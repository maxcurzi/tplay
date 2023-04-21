//! Provides functionality to open and iterate over different media types.
//!
//! This module contains the `FrameIterator` enum and its associated functions for handling
//! different media types such as images, videos, and animated GIFs. It also includes helper
//! functions to open and process media files, as well as downloading and opening YouTube videos.
use crate::{audio::utils::has_audio, common::errors::*, downloader::youtube};
use gif;
use image::{io::Reader as ImageReader, DynamicImage, ImageBuffer};
use opencv::{imgproc, prelude::*, videoio::VideoCapture};
use serde_json::Value;
use std::{
    fs::File,
    io,
    path::Path,
    process::{Command, Stdio},
};
use url::Url;

/// An iterator over the frames of a media file.
///
/// This enum represents an iterator over the frames of different types of media files, including
/// static images, videos, and animated GIFs. Each variant of the enum corresponds to a different
/// media type.
///
/// # Variants
///
/// * `Image` - Represents a single-frame static image. Contains an `Option<DynamicImage>`.
/// * `Video` - Represents a video file. Contains a `VideoCapture` object.
/// * `AnimatedGif` - Represents an animated GIF file. Contains a vector of `DynamicImage` frames
///   and the index of the current frame.
pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedGif {
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
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
            // eprintln!("Could not decode frame");
        }
    }

    Ok(FrameIterator::AnimatedGif {
        frames,
        current_frame: 0,
    })
}

fn extract_fps(video_path: &str) -> Result<f64, Box<dyn std::error::Error>> {
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
        .output()?;

    let output_str = String::from_utf8(output.stdout)?;
    let json_value: Value = serde_json::from_str(&output_str)?;
    let r_frame_rate = json_value["streams"][0]["r_frame_rate"]
        .as_str()
        .ok_or("Failed to get r_frame_rate")?;

    let (num, denom) = {
        let mut iter = r_frame_rate.split('/');
        (
            iter.next()
                .ok_or("Invalid r_frame_rate format")?
                .parse::<f64>()?,
            iter.next()
                .ok_or("Invalid r_frame_rate format")?
                .parse::<f64>()?,
        )
    };

    let fps = num / denom;
    Ok(fps)
}

/// Opens the specified media file and returns a `FrameIterator` for iterating over its frames.
///
/// This function accepts a path to a media file and determines its type based on its extension. It
/// supports images (PNG, BMP, ICO, TIF, TIFF, JPG, JPEG), videos (MP4, AVI, WEBM, MKV, MOV, FLV,
/// OGG), and animated GIFs. If the provided path is a URL pointing to a YouTube video, the video
/// will be downloaded and opened.
///
/// # Arguments
///
/// * `path` - A reference to a path or a URL of the media file.
///
/// # Returns
///
/// A `Result` containing a `FrameIterator` if the media file is successfully opened, or a `MyError`
/// if an error occurs.
pub fn open_media<P: AsRef<Path>>(path: &P) -> Result<(FrameIterator, bool, Option<f64>), MyError> {
    let path = path.as_ref();
    let ext = path.extension().and_then(std::ffi::OsStr::to_str);
    let mut fps = None;

    if let Ok(fpsf) = extract_fps(path.as_os_str().to_str().unwrap_or("")) {
        // println!("FPS: {}", fps);
        fps = Some(fpsf);
    }
    let audio = has_audio(path.as_os_str().to_str().unwrap_or(""));

    // Check if the path is a URL and has a YouTube domain
    if let Ok(url) = Url::parse(path.to_str().unwrap_or("")) {
        if let Some(domain) = url.domain() {
            if domain.ends_with("youtube.com") || domain.ends_with("youtu.be") {
                let video = youtube::download_video(path.to_str().unwrap_or(""))?;
                return Ok((open_video(&video)?, audio.is_ok(), fps));
            }
        }
    }

    match ext {
        Some("png") | Some("bmp") | Some("ico") | Some("tif") | Some("tiff") | Some("jpg")
        | Some("jpeg") => Ok((open_image(path)?, false, None)),
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") | Some("mov") | Some("flv")
        | Some("ogg") => Ok((open_video(path)?, audio.is_ok(), fps)),
        Some("gif") => Ok((open_gif(path)?, false, None)),
        _ => Ok((open_video(path)?, audio.is_ok(), fps)), // Unknown extension, try to open as video anyway
    }
}
