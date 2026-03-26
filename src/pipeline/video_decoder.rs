//! FFmpeg-based video decoder that replaces OpenCV's VideoCapture.
//!
//! Provides frame-by-frame video decoding, seeking, and position tracking
//! using the `ffmpeg-next` crate (Rust bindings to libavformat/libavcodec/libswscale).

use crate::common::errors::*;
use image::{DynamicImage, ImageBuffer, Rgb};

use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{input, input_with_dictionary, Pixel};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video as FfmpegFrame;
use ffmpeg_next::Dictionary;

use std::sync::Once;

static FFMPEG_INIT: Once = Once::new();

fn ensure_ffmpeg_init() {
    FFMPEG_INIT.call_once(|| {
        ffmpeg::init().expect("Failed to initialize ffmpeg");
        // Suppress ffmpeg's verbose logging (HLS segment fetches, etc.)
        // to avoid dumping noise to stderr on exit.
        ffmpeg::log::set_level(ffmpeg::log::Level::Fatal);
    });
}

/// An FFmpeg-based video decoder providing frame iteration, seeking, and position tracking.
#[allow(dead_code)]
pub struct VideoDecoder {
    input_ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    scaler: ScalingContext,
    /// Total number of frames (estimated from stream metadata, may be 0 if unknown).
    total_frames: i64,
    /// Stream time base as a rational number (for timestamp conversion).
    time_base: ffmpeg::Rational,
    /// Duration of the stream in time_base units.
    stream_duration: i64,
    /// Current frame position (0-based), tracked manually.
    current_frame: i64,
    /// FPS of the video stream.
    fps: f64,
    /// Whether we've reached EOF.
    eof: bool,
    /// Whether the source is a network stream (affects seek behavior).
    is_streaming: bool,
}

// SAFETY: The raw pointers in ffmpeg-next's ScalingContext and decoder contexts
// are only accessed from one thread at a time (VideoDecoder is not Clone and is
// moved into exactly one thread). FFmpeg's decoding API is safe for single-threaded use.
unsafe impl Send for VideoDecoder {}

impl VideoDecoder {
    /// Opens a video file and prepares the decoder.
    pub fn open(path: &str) -> Result<Self, MyError> {
        ensure_ffmpeg_init();

        let is_streaming = path.starts_with("http://") || path.starts_with("https://");

        let input_ctx = if is_streaming {
            let mut opts = Dictionary::new();
            // Buffer up to 5 MB before starting playback to reduce stutter
            opts.set("buffer_size", "5242880");
            // Allow up to 10 seconds of analysis to detect streams properly
            opts.set("analyzeduration", "10000000");
            // Increase probe size for better format detection
            opts.set("probesize", "5000000");
            input_with_dictionary(&path, opts)
        } else {
            input(&path)
        }
        .map_err(|e| {
            MyError::Application(format!("{}: {} ({:?})", ERROR_OPENING_VIDEO, path, e))
        })?;

        let stream = input_ctx.streams().best(Type::Video).ok_or_else(|| {
            MyError::Application(format!("{}: no video stream", ERROR_OPENING_VIDEO))
        })?;

        let video_stream_index = stream.index();
        let time_base = stream.time_base();
        let stream_duration = stream.duration();
        let total_frames = stream.frames();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .map_err(|e| MyError::Application(format!("{}: {:?}", ERROR_OPENING_VIDEO, e)))?;

        let decoder = context_decoder
            .decoder()
            .video()
            .map_err(|e| MyError::Application(format!("{}: {:?}", ERROR_OPENING_VIDEO, e)))?;

        let fps = {
            let r = stream.avg_frame_rate();
            if r.denominator() != 0 {
                r.numerator() as f64 / r.denominator() as f64
            } else {
                30.0 // fallback
            }
        };

        let scaler = ScalingContext::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )
        .map_err(|e| {
            MyError::Application(format!(
                "{}: failed to create scaler ({:?})",
                ERROR_OPENING_VIDEO, e
            ))
        })?;

        Ok(Self {
            input_ctx,
            video_stream_index,
            decoder,
            scaler,
            total_frames,
            time_base,
            stream_duration,
            current_frame: 0,
            fps,
            eof: false,
            is_streaming,
        })
    }

    /// Returns the FPS of the video.
    #[allow(dead_code)]
    pub fn fps(&self) -> f64 {
        self.fps
    }

    /// Returns the source video dimensions (width, height).
    pub fn dimensions(&self) -> (u32, u32) {
        (self.decoder.width(), self.decoder.height())
    }

    /// Returns whether the source is a network stream.
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    /// Decodes and returns the next frame as a `DynamicImage`.
    pub fn next_frame(&mut self) -> Option<DynamicImage> {
        if self.eof {
            return None;
        }
        // Try to receive already-buffered decoded frames first
        if let Some(img) = self.receive_frame() {
            self.current_frame += 1;
            return Some(img);
        }
        // Feed packets until we get a frame or reach EOF
        loop {
            match self.next_video_packet() {
                Some(packet) => {
                    if self.decoder.send_packet(&packet).is_err() {
                        continue;
                    }
                    if let Some(img) = self.receive_frame() {
                        self.current_frame += 1;
                        return Some(img);
                    }
                }
                None => {
                    // EOF — flush the decoder
                    let _ = self.decoder.send_eof();
                    let img = self.receive_frame();
                    if img.is_some() {
                        self.current_frame += 1;
                    }
                    self.eof = true;
                    return img;
                }
            }
        }
    }

    /// Receive a decoded frame from the decoder and convert to DynamicImage.
    fn receive_frame(&mut self) -> Option<DynamicImage> {
        let mut decoded = FfmpegFrame::empty();
        if self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = FfmpegFrame::empty();
            self.scaler.run(&decoded, &mut rgb_frame).ok()?;
            frame_to_image(&rgb_frame)
        } else {
            None
        }
    }

    /// Get the next packet belonging to the video stream.
    fn next_video_packet(&mut self) -> Option<ffmpeg::Packet> {
        loop {
            let mut packet_iter = self.input_ctx.packets();
            match packet_iter.next() {
                Some((stream, packet)) => {
                    if stream.index() == self.video_stream_index {
                        return Some(packet);
                    }
                    // skip non-video packets
                }
                None => return None,
            }
        }
    }

    /// Skips `n` frames by decoding (and discarding) them.
    pub fn skip_frames(&mut self, n: usize) {
        for _ in 0..n {
            if self.next_frame().is_none() {
                break;
            }
        }
    }

    /// Resets playback to the beginning.
    pub fn reset(&mut self) {
        // Seek to the very beginning
        let _ = self.input_ctx.seek(0, ..i64::MAX);
        self.decoder.flush();
        self.current_frame = 0;
        self.eof = false;
    }

    /// Returns whether the decoder has reached the end of the stream.
    pub fn is_at_end(&self) -> bool {
        self.eof
    }

    /// Seeks by `seconds` relative to the current position.
    /// Positive values seek forward, negative values seek backward.
    /// Returns `true` if the seek was (at least partially) successful.
    pub fn seek_seconds(&mut self, seconds: f64) -> bool {
        // Compute current position in seconds
        let current_secs = self.current_frame as f64 / self.fps;
        let target_secs = (current_secs + seconds).max(0.0);

        self.seek_to_seconds(target_secs)
    }

    /// Seeks to an absolute position in seconds.
    ///
    /// After the keyframe seek, decodes and discards frames up to the target
    /// position so that the next call to `next_frame()` returns the frame at
    /// (or just past) the requested time — matching the old OpenCV behaviour
    /// and keeping `current_frame` accurate for A/V sync.
    fn seek_to_seconds(&mut self, target_secs: f64) -> bool {
        // Network streams (HLS etc.) don't support reliable seeking via
        // avformat_seek_file — the only way to advance is sequential reading.
        // Return false so the caller knows seeking didn't happen; the sync
        // logic will catch up by skipping frames instead.
        if self.is_streaming {
            return false;
        }

        // Convert target seconds to stream time_base units
        let target_ts = (target_secs * self.time_base.denominator() as f64
            / self.time_base.numerator() as f64) as i64;

        // Seek to the nearest keyframe at or before target_ts.
        let result = self.input_ctx.seek(target_ts, ..target_ts + 1).is_ok();

        if result {
            self.decoder.flush();
            self.eof = false;

            // For local files, decode forward from the keyframe to the exact
            // target for frame-accurate seeking.
            self.decode_forward_to(target_ts);
        }
        result
    }

    /// Decode and discard frames until the next frame's PTS reaches `target_ts`.
    /// Updates `current_frame` from the PTS of the last consumed frame so that
    /// the position counter stays accurate after a seek.
    fn decode_forward_to(&mut self, target_ts: i64) {
        loop {
            match self.next_video_packet() {
                Some(packet) => {
                    if self.decoder.send_packet(&packet).is_err() {
                        continue;
                    }
                    let mut decoded = FfmpegFrame::empty();
                    while self.decoder.receive_frame(&mut decoded).is_ok() {
                        let pts = decoded.pts().unwrap_or(0);
                        // Update current_frame from PTS
                        let secs = pts as f64 * self.time_base.numerator() as f64
                            / self.time_base.denominator() as f64;
                        self.current_frame = (secs * self.fps).round() as i64;

                        if pts >= target_ts {
                            return;
                        }
                    }
                }
                None => {
                    // Hit EOF while decoding forward
                    self.eof = true;
                    return;
                }
            }
        }
    }

    /// Seeks to a specific frame index (0-based).
    pub fn seek_to_frame(&mut self, target_frame: usize) {
        let target_secs = target_frame as f64 / self.fps;
        self.seek_to_seconds(target_secs);
    }

    /// Returns the current frame position (0-based).
    pub fn get_position_frames(&self) -> i64 {
        self.current_frame
    }
}

/// Converts an RGB24 FFmpeg frame to a `DynamicImage`.
fn frame_to_image(frame: &FfmpegFrame) -> Option<DynamicImage> {
    let width = frame.width();
    let height = frame.height();
    let stride = frame.stride(0);
    let data = frame.data(0);

    // Build pixel buffer, handling stride != width*3
    let row_bytes = (width * 3) as usize;
    let mut pixels = Vec::with_capacity(row_bytes * height as usize);
    for y in 0..height as usize {
        let row_start = y * stride;
        pixels.extend_from_slice(&data[row_start..row_start + row_bytes]);
    }

    ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, pixels).map(DynamicImage::ImageRgb8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::NamedTempFile;

    /// Creates a tiny synthetic test video (10 frames, 16x16, 10fps, red solid color).
    /// Returns a NamedTempFile that keeps the file alive for the duration of the test.
    fn create_test_video() -> NamedTempFile {
        let tmp = tempfile::Builder::new()
            .suffix(".mp4")
            .tempfile()
            .expect("Failed to create temp file");
        let path = tmp.path().to_str().unwrap().to_string();

        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=16x16:r=10:d=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-t",
                "1",
                &path,
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("Failed to run ffmpeg to create test video");

        assert!(status.success(), "ffmpeg failed to create test video");
        tmp
    }

    #[test]
    fn test_open_invalid_path_returns_error() {
        let result = VideoDecoder::open("/nonexistent/path/to/video.mp4");
        assert!(result.is_err());
    }

    #[test]
    fn test_open_valid_video() {
        let tmp = create_test_video();
        let decoder = VideoDecoder::open(tmp.path().to_str().unwrap());
        assert!(decoder.is_ok());
        let decoder = decoder.unwrap();
        assert!(decoder.fps() > 0.0);
        assert!(!decoder.is_at_end());
        assert_eq!(decoder.get_position_frames(), 0);
    }

    #[test]
    fn test_next_frame_returns_valid_image() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        let frame = decoder.next_frame();
        assert!(frame.is_some());
        let img = frame.unwrap();
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);
        assert_eq!(decoder.get_position_frames(), 1);
    }

    #[test]
    fn test_next_frame_rgb_values() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        let frame = decoder.next_frame().unwrap();
        let rgb = frame.to_rgb8();
        let pixel = rgb.get_pixel(8, 8);
        // Red video: R should be high, G and B should be low
        // (not exact 255/0/0 due to YUV420 conversion, but clearly red)
        assert!(
            pixel[0] > 200,
            "Red channel should be high, got {}",
            pixel[0]
        );
        assert!(
            pixel[1] < 50,
            "Green channel should be low, got {}",
            pixel[1]
        );
        assert!(
            pixel[2] < 50,
            "Blue channel should be low, got {}",
            pixel[2]
        );
    }

    #[test]
    fn test_skip_frames_advances_position() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        decoder.skip_frames(3);
        assert_eq!(decoder.get_position_frames(), 3);
    }

    #[test]
    fn test_eof_after_all_frames() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        // Exhaust all frames (10fps * 1s = ~10 frames)
        while decoder.next_frame().is_some() {}
        assert!(decoder.is_at_end());
        assert!(decoder.next_frame().is_none());
    }

    #[test]
    fn test_reset_returns_to_start() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        // Read a few frames
        for _ in 0..5 {
            decoder.next_frame();
        }
        assert!(decoder.get_position_frames() > 0);

        decoder.reset();
        assert_eq!(decoder.get_position_frames(), 0);
        assert!(!decoder.is_at_end());

        // Should be able to read frames again
        let frame = decoder.next_frame();
        assert!(frame.is_some());
    }

    #[test]
    fn test_seek_seconds_forward() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        let result = decoder.seek_seconds(0.5);
        assert!(result);
        assert!(!decoder.is_at_end());
        // Should be able to read a frame after seeking
        let frame = decoder.next_frame();
        assert!(frame.is_some());
    }

    #[test]
    fn test_seek_seconds_backward_clamps_to_zero() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        // Read some frames first
        for _ in 0..5 {
            decoder.next_frame();
        }
        // Seek backward further than current position
        let result = decoder.seek_seconds(-100.0);
        assert!(result);
        assert_eq!(decoder.get_position_frames(), 0);
    }

    #[test]
    fn test_seek_to_frame() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        decoder.seek_to_frame(5);
        assert!(!decoder.is_at_end());
        let frame = decoder.next_frame();
        assert!(frame.is_some());
    }

    #[test]
    fn test_reset_after_eof() {
        let tmp = create_test_video();
        let mut decoder = VideoDecoder::open(tmp.path().to_str().unwrap()).unwrap();
        // Exhaust all frames
        while decoder.next_frame().is_some() {}
        assert!(decoder.is_at_end());

        // Reset and verify we can play again
        decoder.reset();
        assert!(!decoder.is_at_end());
        assert_eq!(decoder.get_position_frames(), 0);
        let frame = decoder.next_frame();
        assert!(frame.is_some());
    }
}
