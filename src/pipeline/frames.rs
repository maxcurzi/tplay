//! Provides functionality to open and iterate over various media types.
//!
//! This module contains the `FrameIterator` enum and its associated functions for handling
//! different media types such as images, videos, and animated GIFs. It also includes helper
//! functions to open and process media files, as well as downloading and opening YouTube videos.
use crate::{
    audio::utils::has_audio,
    common::{errors::*, utils::*},
    downloader::youtube,
};
use either::Either;
use gif;
use image::{DynamicImage, ImageReader};
use libwebp_sys as webp;
use opencv::{prelude::*, videoio::VideoCapture};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use tempfile::{tempdir, TempPath};
use url::Url;

/// An iterator over the frames of a media file.
///
/// This enum represents an iterator for different types of media files, such as
/// static images, videos, and animated GIFs/WEBPs.
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
    AnimatedImage {
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
            FrameIterator::AnimatedImage {
                ref frames,
                ref mut current_frame,
            } => {
                if *current_frame == frames.len() {
                    None
                } else {
                    let frame = frames.get(*current_frame).cloned();
                    *current_frame += 1;
                    frame
                }
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
            FrameIterator::AnimatedImage {
                ref mut current_frame,
                frames,
            } => {
                *current_frame = (*current_frame + n) % frames.len();
            }
        }
    }

    pub fn reset(&mut self) {
        match self {
            FrameIterator::Image(_) => {
                // For a single image, reset is a no-op, since there's only one frame
            }
            FrameIterator::Video(ref mut video) => {
                let _ = video.set(opencv::videoio::CAP_PROP_POS_AVI_RATIO, 0.0);
            }
            FrameIterator::AnimatedImage {
                ref mut current_frame,
                ..
            } => {
                *current_frame = 0;
            }
        }
    }
}

/// Opens the specified media file and returns a `FrameIterator` for iterating over its frames.
///
/// This function takes a path or downloadable URL to a media file and identifies its type based on the file extension.
/// It supports images (PNG, BMP, ICO, TIF, TIFF, JPG, JPEG), videos (MP4, AVI, WEBM, MKV, MOV, FLV,
/// OGG), and animated GIFs/WEBPs. If the URL pointing to a YouTube video, the content will be handled in a custom manner.
///
/// # Arguments
///
/// * `path` - A reference to a path or a URL of the media file.
///
/// # Returns
///
/// A `Result` containing a `FrameData` struct if the media file is successfully opened, or a
/// `MyError` if an error occurs.
pub fn open_media(path: String, broswer: String) -> Result<MediaData, MyError> {
    // Check if the path is a URL
    if let Ok(url) = Url::parse(path.as_str()) {
        if let Some(domain) = url.domain() {
            // handle YouTube domains specially
            if domain.ends_with("youtube.com") || domain.ends_with("youtu.be") {
                let video = youtube::download_video(path.as_str(), broswer.as_str())?;
                let fps = extract_fps(video.as_os_str().to_str().unwrap_or(""));
                let video_open = open_video(&video)?;
                return Ok(MediaData {
                    frame_iter: video_open,
                    fps,
                    audio_path: Some(Either::Left(video)),
                });
            } else {
                // otherwise download the url to a temp file and open media from there.
                let tmp = tempdir()?;
                // use the last segment of the url path (for the ext) or a random name otherwise with no extension
                let name = url
                    .path_segments()
                    .and_then(|s| s.last())
                    .unwrap_or("unknown_media");
                let p = tmp.path().join(name);
                download_url_to_file(p.as_path(), url)?;
                open_media_from_path(p.as_os_str().to_str().unwrap_or(""), p.as_path())
            }
        } else {
            open_media_from_path(path.as_str(), &Path::new(path.as_str()))
        }
    } else {
        open_media_from_path(path.as_str(), &Path::new(path.as_str()))
    }
}

/// Opens the media file from a local path and returns a `FrameIterator` for iterating over its frames.
///
/// This function is called from open_media
///
/// # Arguments
///
/// * `path_str` - A reference to the path str.
/// * `path` - A reference to a corresponding Path structure.
///
/// # Returns
///
/// A `Result` containing a `FrameData` struct if the media file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_media_from_path(path_str: &str, path: &Path) -> Result<MediaData, MyError> {
    let fps = extract_fps(path_str);
    let audio = has_audio(path_str)?;
    let audio_track = if audio {
        Some(Either::Right(path_str.to_owned()))
    } else {
        None
    };

    let ext = path.extension().and_then(std::ffi::OsStr::to_str);
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
        Some("gif") => {
            let (frame_iter, fps) = open_gif(path)?;
            Ok(MediaData {
                frame_iter,
                fps: Some(fps),
                audio_path: None,
            })
        }

        // Webp
        Some("webp") => {
            let (frame_iter, fps) = open_webp(path)?;
            Ok(MediaData {
                frame_iter,
                fps: Some(fps),
                audio_path: None,
            })
        }

        // Unknown extension, try open as video
        _ => Ok(MediaData {
            frame_iter: open_video(path)?,
            fps,
            audio_path: audio_track,
        }),
    }
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

/// Writes the content downloaded from a url to a file.
///
//
/// # Arguments
///
/// * `path` - A path to the file to be written
/// * `url` - The url from which to download the file content
///
/// # Returns
///
/// A `Result`
/// `MyError` if an error occurs.
fn download_url_to_file(path: &Path, url: Url) -> Result<(), MyError> {
    let tmp_file = File::create(path)?;
    let mut tmp_file = std::io::BufWriter::new(tmp_file);
    tmp_file
        .write_all(
            reqwest::blocking::get(url)
                .and_then(|resp| resp.bytes())
                .map_err(|err| {
                    MyError::Application(format!(
                        "{error}: {err:?}",
                        error = ERROR_DOWNLOADING_RESOURCE
                    ))
                })?
                .as_ref(),
        )
        .map_err(|err| {
            MyError::Application(format!("{error}: {err:?}", error = ERROR_OPENING_RESOURCE))
        })
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
/// A `Result` containing a `FrameIterator` and fps if the animated GIF file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_gif(path: &Path) -> Result<(FrameIterator, f64), MyError> {
    let file = File::open(path).map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_RESOURCE))
    })?;
    let mut options = gif::DecodeOptions::new();
    // https://lib.rs/crates/gif-dispose
    // for gif_dispose frame composing for rgba output, we need to set this as indexed.
    options.set_color_output(gif::ColorOutput::Indexed);
    let mut decoder = options.read_info(file).map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_READING_GIF_HEADER))
    })?;

    // delay is in units of 10ms, so we'll divide by 100.0, not 1000.0
    let mut delay: u64 = 0;
    let mut frames = Vec::new();
    // The gif crate only exposes raw frame data that is not sufficient to render animated GIFs properly.
    // GIF requires special composing of frames which is non-trivial.
    let mut screen = gif_dispose::Screen::new_decoder(&decoder);
    while let Ok(Some(frame)) = decoder.read_next_frame() {
        delay += frame.delay as u64;
        screen.blit_frame(&frame).map_err(|e| {
            MyError::Application(format!("{error}: {e:?}", error = ERROR_DECODING_IMAGE))
        })?;
        let (buf, width, height) = screen.pixels_rgba().to_contiguous_buf();
        frames.push(DynamicImage::ImageRgba8(image::RgbaImage::from_fn(
            width as u32,
            height as u32,
            |x, y| {
                let rgba = buf.as_ref()[y as usize * width + x as usize];
                image::Rgba([rgba.r, rgba.g, rgba.b, rgba.a])
            },
        )));
    }

    // fps is only an average across all frames, there is no per frame delay modelling
    let fps = frames.len() as f64 / (delay.max(1) as f64 / 100.0);
    Ok((
        FrameIterator::AnimatedImage {
            frames,
            current_frame: 0,
        },
        fps,
    ))
}

/// Opens the specified animated WEBP file and returns a `FrameIterator`.
///
/// This helper function opens an animated WEBP file and creates a `FrameIterator::AnimatedWebp`
/// variant containing all the frames of the animation.
///
/// # Arguments
///
/// * `path` - A reference to the path of the animated WEBP file.
///
/// # Returns
///
/// A `Result` containing a `FrameIterator` and fps if the animated WEBP file is successfully opened, or a
/// `MyError` if an error occurs.
fn open_webp(path: &Path) -> Result<(FrameIterator, f64), MyError> {
    let mut file = File::open(path).map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_RESOURCE))
    })?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let mut frames = Vec::new();
    let mut first_timestamp: i32 = i32::MAX;
    let mut last_timestamp: i32 = i32::MIN;
    // this code is based on the code example here:
    // https://developers.google.com/speed/webp/docs/container-api#webpanimdecoder_api
    unsafe {
        let mut options = webp::WebPAnimDecoderOptions {
            color_mode: webp::WEBP_CSP_MODE::MODE_RGBA,
            use_threads: 0,
            padding: [0, 0, 0, 0, 0, 0, 0],
        };
        webp::WebPAnimDecoderOptionsInit(&mut options);
        let dec = webp::WebPAnimDecoderNew(
            &webp::WebPData {
                bytes: buf.as_ptr(),
                size: buf.len(),
            },
            &options,
        );
        let mut info = webp::WebPAnimInfo::default();
        webp::WebPAnimDecoderGetInfo(dec, &mut info);
        let frame_sz = (info.canvas_width * info.canvas_height * 4) as usize;
        for _ in 0..info.loop_count {
            while webp::WebPAnimDecoderHasMoreFrames(dec) != 0 {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut timestamp: i32 = 0;
                webp::WebPAnimDecoderGetNext(dec, &mut buf, &mut timestamp);
                first_timestamp = first_timestamp.min(timestamp);
                last_timestamp = last_timestamp.max(timestamp);
                if let Some(image) = image::RgbaImage::from_raw(
                    info.canvas_width,
                    info.canvas_height,
                    std::slice::from_raw_parts(buf, frame_sz).to_vec(),
                ) {
                    frames.push(DynamicImage::ImageRgba8(image));
                } else {
                    // eprintln!("Failed to decode frame");
                }
            }
            webp::WebPAnimDecoderReset(dec);
        }
        webp::WebPAnimDecoderDelete(dec);
    }

    // fps is only an average across all frames, there is no per frame delay modelling
    let fps = frames.len() as f64
        / ((last_timestamp.saturating_sub(first_timestamp).max(1)) as f64 / 1000.0);
    Ok((
        FrameIterator::AnimatedImage {
            frames,
            current_frame: 0,
        },
        fps,
    ))
}
