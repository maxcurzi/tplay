//! FFmpeg-based video decoder that replaces OpenCV's VideoCapture.
//!
//! Provides frame-by-frame video decoding, seeking, and position tracking
//! using the `ffmpeg-next` crate (Rust bindings to libavformat/libavcodec/libswscale).

use crate::common::errors::*;
use image::{DynamicImage, ImageBuffer, Rgb};

use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{input, Pixel};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video as FfmpegFrame;

use std::sync::Once;

static FFMPEG_INIT: Once = Once::new();

fn ensure_ffmpeg_init() {
    FFMPEG_INIT.call_once(|| {
        ffmpeg::init().expect("Failed to initialize ffmpeg");
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
}

// SAFETY: The raw pointers in ffmpeg-next's ScalingContext and decoder contexts
// are only accessed from one thread at a time (VideoDecoder is not Clone and is
// moved into exactly one thread). FFmpeg's decoding API is safe for single-threaded use.
unsafe impl Send for VideoDecoder {}

impl VideoDecoder {
    /// Opens a video file and prepares the decoder.
    pub fn open(path: &str) -> Result<Self, MyError> {
        ensure_ffmpeg_init();

        let input_ctx = input(&path).map_err(|e| {
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
        })
    }

    /// Returns the FPS of the video.
    #[allow(dead_code)]
    pub fn fps(&self) -> f64 {
        self.fps
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
    fn seek_to_seconds(&mut self, target_secs: f64) -> bool {
        // Convert target seconds to stream time_base units
        let target_ts = (target_secs * self.time_base.denominator() as f64
            / self.time_base.numerator() as f64) as i64;

        // Use a range-based seek: allow seeking slightly before target
        let min_ts = target_ts.saturating_sub(
            (self.time_base.denominator() as i64) / self.time_base.numerator().max(1) as i64,
        );

        let result = self.input_ctx.seek(min_ts, ..target_ts).is_ok();

        if result {
            self.decoder.flush();
            self.current_frame = (target_secs * self.fps).round() as i64;
            self.eof = false;
        }
        result
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
