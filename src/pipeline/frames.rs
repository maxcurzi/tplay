//! Provides functionality to open and iterate over various media types.
//!
//! This module contains the `FrameIterator` enum and its associated functions for handling
//! different media types such as images, videos, and animated GIFs. It also includes helper
//! functions to open and process media files, as well as downloading and opening YouTube videos.
use crate::{audio::utils::has_audio, common::errors::*, downloader::youtube};
use either::Either;
use gif;
use image::{io::Reader as ImageReader, DynamicImage, ImageBuffer};
use num::{Rational64, ToPrimitive};
use opencv::{imgproc, prelude::*, videoio::VideoCapture};
use serde_json::Value;
use std::{
    fs::File,
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};
use tempfile::TempPath;
use url::Url;

/// An iterator over the frames of a media file.
///
/// This enum represents an iterator for different types of media files, such as
/// static images, videos, and animated GIFs.
///
/// # Variants
///
/// * `Image` - Represents a single-frame static image. Contains an
///   `Option<DynamicImage>`.
/// * `Video` - Represents a video file. Contains a `VideoCapture` object.
/// * `AnimatedGif` - Represents an animated GIF file. Contains a vector of
///   `DynamicImage` frames and the index of the current frame.
pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedGif {
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
}

/// A named struct for storing the data returned by `open_media`.
///
/// # Fields
///
/// * `frame_iter` - A `FrameIterator` for iterating over the frames of the media file.
/// * `fps` - The frame rate of the media file, if available.
/// * `audio_path` - The path to the audio track of the media file, if available.
pub struct MediaData {
    pub frame_iter: FrameIterator,
    pub fps: Option<f64>,
    pub audio_path: Option<Either<TempPath, String>>,
}

/// Implements the `Iterator` trait for `FrameIterator`.
///
/// This implementation allows `FrameIterator` to iterate over the frames of the media file it
/// represents, returning a `DynamicImage` for each frame. The behavior of the `next()` method
/// depends on the variant of the `FrameIterator`:
///
/// * `Image` - Returns the single `DynamicImage` and sets the `Option` to `None`.
/// * `Video` - Captures and returns the next video frame as a grayscale `DynamicImage`.
/// * `AnimatedGif` - Returns the next frame in the animation sequence as a `DynamicImage`.
impl Iterator for FrameIterator {
    type Item = DynamicImage;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FrameIterator::Image(ref mut img) => img.take(),
            FrameIterator::Video(ref mut video) => capture_video_frame(video),
            FrameIterator::AnimatedGif {
                ref frames,
                ref mut current_frame,
            } => {
                let frame = frames.get(*current_frame).cloned();
                *current_frame = (*current_frame + 1) % frames.len();
                frame
            }
        }
    }
}

impl FrameIterator {
    /// Skips the specified number of frames.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of frames to skip.
    ///
    /// # Returns
    ///
    /// A relevant FrameIterator.
    pub fn skip_frames(&mut self, n: usize) {
        match self {
            FrameIterator::Image(_) => {
                // For a single image, skipping is a no-op, since there's only one frame
            }
            FrameIterator::Video(ref mut video) => {
                for _ in 0..n {
                    let mut frame = Mat::default();
                    if !video.read(&mut frame).unwrap_or(false) || frame.empty() {
                        break;
                    }
                }
            }
            FrameIterator::AnimatedGif {
                ref mut current_frame,
                frames,
            } => {
                *current_frame = (*current_frame + n) % frames.len();
            }
        }
    }
}

/// Converts an opencv Mat frame to a dynamic image.
///
/// This helper function takes a reference to a video frame in BGR format and returns an optional
/// `DynamicImage`.
///
/// # Arguments
///
/// * `mat` - A reference to a `Mat` object containing the video frame.
///
/// # Returns
///
/// An `Option` containing a `DynamicImage` if the frame is successfully converted, or
/// `None` if an error occurs.
fn mat_to_dynamic_image(mat: &Mat) -> Option<DynamicImage> {
    let mut rgb_mat = Mat::default();
    if imgproc::cvt_color(&mat, &mut rgb_mat, imgproc::COLOR_BGR2RGB, 0).is_ok() {
        if let Ok(_elem_size) = rgb_mat.elem_size() {
            if let Ok(size) = rgb_mat.size() {
                let reshaped_mat = rgb_mat.reshape(1, size.width * size.height).ok()?;
                let data_vec: Vec<u8> = reshaped_mat
                    .data_typed::<u8>()
                    .expect("Unexpected invalid data")
                    .to_vec();

                if let Some(img_buf) = ImageBuffer::<image::Rgb<u8>, _>::from_raw(
                    size.width as u32,
                    size.height as u32,
                    data_vec,
                ) {
                    return Some(DynamicImage::ImageRgb8(img_buf));
                }
            }
        }
    }
    None
}

/// Captures the next video frame as a dynamic image.
///
/// This helper function reads the next frame from the provided video and converts it into a
/// `DynamicImage`.
///
/// # Arguments
///
/// * `video` - A mutable reference to a `VideoCapture` object.
///
/// # Returns
///
/// An `Option` containing a `DynamicImage` if the frame is successfully captured and
/// converted, or `None` if an error occurs or the video has ended.
fn capture_video_frame(video: &mut VideoCapture) -> Option<DynamicImage> {
    let mut frame = Mat::default();
    if video.read(&mut frame).unwrap_or(false) && !frame.empty() {
        mat_to_dynamic_image(&frame)
    } else {
        None
    }
}

/// Opens the specified image file and returns a `FrameIterator`.
///
/// This helper function opens an image file and creates a `FrameIterator::Image` variant.
///
/// # Arguments
///
/// * `path` - A reference to the path of the image file.
///
/// # Returns
///
/// A `Result` containing a `FrameIterator` if the image file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_image(path: &Path) -> Result<FrameIterator, MyError> {
    let img = ImageReader::open(path)?.decode().map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_DECODING_IMAGE))
    })?;
    Ok(FrameIterator::Image(Some(img)))
}

/// Opens the specified video file and returns a `FrameIterator`.
///
/// This helper function opens a video file and creates a `FrameIterator::Video` variant.
///
/// # Arguments
///
/// * `path` - A reference to the path of the video file.
///
/// # Returns
///
/// A `Result` containing a `FrameIterator` if the video file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_video(path: &Path) -> Result<FrameIterator, MyError> {
    let video = VideoCapture::from_file(
        path.to_str().expect(ERROR_OPENING_VIDEO),
        opencv::videoio::CAP_ANY,
    )?;

    if video.is_opened()? {
        Ok(FrameIterator::Video(video))
    } else {
        Err(MyError::Application(ERROR_OPENING_VIDEO.to_string()))
    }
}

/// Opens the specified animated GIF file and returns a `FrameIterator`.
///
/// This helper function opens an animated GIF file and creates a `FrameIterator::AnimatedGif`
/// variant containing all the frames of the animation.
///
/// # Arguments
///
/// * `path` - A reference to the path of the animated GIF file.
///
/// # Returns
///
/// A `Result` containing a `FrameIterator` if the animated GIF file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_gif(path: &Path) -> Result<FrameIterator, MyError> {
    let file = File::open(path)
        .map_err(|e| MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_GIF)))?;
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(file).map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_READING_GIF_HEADER))
    })?;

    let mut frames = Vec::new();
    while let Ok(Some(frame)) = decoder.read_next_frame() {
        let buffer = frame.buffer.clone();
        if let Some(image) = image::RgbaImage::from_raw(
            decoder.width() as u32,
            decoder.height() as u32,
            buffer.to_vec(),
        ) {
            frames.push(DynamicImage::ImageRgba8(image));
        } else {
            // eprintln!("Failed to decode frame");
        }
    }

    Ok(FrameIterator::AnimatedGif {
        frames,
        current_frame: 0,
    })
}

/// Extracts the frame rate from a video file using `ffprobe`.
///
/// # Arguments
///
/// * `video_path` - A reference to the path of the video file.
///
/// # Returns
///
/// An `Option` containing the frame rate if the frame rate is successfully
/// extracted, or `None` if an error occurs.
fn extract_fps(video_path: &str) -> Option<f64> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0")
        .arg("-show_entries")
        .arg("stream=r_frame_rate")
        .arg("-of")
        .arg("json")
        .arg(video_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to extract fps from video. Is ffprobe installed?");

    let output_str = String::from_utf8(output.stdout).unwrap_or("".to_string());
    let json_value: Value = serde_json::from_str(&output_str).unwrap_or(Value::Null);
    if json_value != Value::Null {
        let r_frame_rate = json_value["streams"][0]["r_frame_rate"]
            .as_str()
            .ok_or("".to_string())
            .unwrap_or("");

        let frame_rate_f = Rational64::from_str(r_frame_rate);
        if let Ok(frame_rate) = frame_rate_f {
            return Some(frame_rate.to_f64().expect("Failed to parse FPS value"));
        }
    }

    None
}

/// Opens the specified media file and returns a `FrameIterator` for iterating over its frames.
///
/// This function takes a path to a media file and identifies its type based on the file extension.
/// It supports images (PNG, BMP, ICO, TIF, TIFF, JPG, JPEG), videos (MP4, AVI, WEBM, MKV, MOV, FLV,
/// OGG), and animated GIFs. If the provided path is a URL pointing to a YouTube video, the video
/// will be downloaded and opened.
///
/// # Arguments
///
/// * `path` - A reference to a path or a URL of the media file.
///
/// # Returns
///
/// A `Result` containing a `FrameData` struct if the media file is successfully opened, or a
/// `MyError` if an error occurs.
pub fn open_media(path: String) -> Result<MediaData, MyError> {
    let p = Path::new(&path);
    let x = Path::new(p).to_owned();
    let path = x.as_path(); //.as_ref();
    let ext = path.extension().and_then(std::ffi::OsStr::to_str);

    // Check if the path is a URL and has a YouTube domain
    if let Ok(url) = Url::parse(path.to_str().unwrap_or("")) {
        if let Some(domain) = url.domain() {
            if domain.ends_with("youtube.com") || domain.ends_with("youtu.be") {
                let video = youtube::download_video(path.to_str().unwrap_or(""))?;
                let fps = extract_fps(video.as_os_str().to_str().unwrap_or(""));
                let video_open = open_video(&video)?;
                return Ok(MediaData {
                    frame_iter: video_open,
                    fps,
                    audio_path: Some(Either::Left(video)),
                });
            }
        }
    }

    let fps = extract_fps(path.as_os_str().to_str().unwrap_or(""));
    let audio = has_audio(path.as_os_str().to_str().unwrap_or(""))?;
    let audio_track = if audio {
        Some(Either::Right(path.to_str().unwrap_or("").to_string()))
    } else {
        None
    };
    match ext {
        // Image extensions
        Some("png") | Some("bmp") | Some("ico") | Some("tif") | Some("tiff") | Some("jpg")
        | Some("jpeg") => Ok(MediaData {
            frame_iter: open_image(path)?,
            fps: None,
            audio_path: None,
        }),

        // Video extensions
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") | Some("mov") | Some("flv")
        | Some("ogg") => Ok(MediaData {
            frame_iter: open_video(path)?,
            fps,
            audio_path: audio_track,
        }),

        // Gif
        Some("gif") => Ok(MediaData {
            frame_iter: open_gif(path)?,
            fps: None,
            audio_path: None,
        }),

        // Unknown extension, try open as video
        _ => Ok(MediaData {
            frame_iter: open_video(path)?,
            fps,
            audio_path: audio_track,
        }),
    }
}
