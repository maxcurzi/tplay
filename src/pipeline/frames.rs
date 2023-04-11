use crate::common::errors::*;
use gif;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer, Rgb};
use opencv::{core::Vec3b, imgproc, prelude::*, videoio::VideoCapture};
use std::fs::File;
use std::path::Path;

pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedGif {
        // decoder: gif::Decoder<File>,
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
}

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

fn capture_video_frame(video: &mut VideoCapture) -> Option<DynamicImage> {
    let mut frame = Mat::default();
    if video.read(&mut frame).unwrap_or(false) && !frame.empty() {
        convert_frame_to_rgb(&frame)
    } else {
        None
    }
}

fn convert_frame_to_rgb(frame: &Mat) -> Option<DynamicImage> {
    let mut rgb_frame = Mat::default();
    if imgproc::cvt_color(&frame, &mut rgb_frame, imgproc::COLOR_BGR2RGB, 0).is_ok() {
        let rows = rgb_frame.rows() as u32;
        let cols = rgb_frame.cols() as u32;

        let img = ImageBuffer::from_fn(cols, rows, |x, y| {
            let pixel: Vec3b = *rgb_frame
                .at_2d(y as i32, x as i32)
                .unwrap_or(&Vec3b::default());
            Rgb([pixel[0], pixel[1], pixel[2]])
        });

        Some(DynamicImage::ImageRgb8(img))
    } else {
        None
    }
}

fn open_image(path: &Path) -> Result<FrameIterator, MyError> {
    let img = ImageReader::open(path)
        .map_err(|e| MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_IMAGE)))?
        .decode()
        .map_err(|e| {
            MyError::Application(format!("{error}: {e:?}", error = ERROR_DECODING_IMAGE))
        })?;
    Ok(FrameIterator::Image(Some(img)))
}

fn open_video(path: &str) -> Result<FrameIterator, MyError> {
    let video = VideoCapture::from_file(path, opencv::videoio::CAP_ANY).map_err(|e| {
        MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_VIDEO))
    })?;

    if video
        .is_opened()
        .map_err(|e| MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_VIDEO)))?
    {
        Ok(FrameIterator::Video(video))
    } else {
        Err(MyError::Application("Could not open the video".to_string()))
    }
}

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

pub fn open_media<P: AsRef<Path>>(path: P) -> Result<FrameIterator, MyError> {
    let path = path.as_ref();
    let ext = path.extension().and_then(std::ffi::OsStr::to_str);

    match ext {
        Some("png") | Some("jpg") | Some("jpeg") => open_image(path),
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") => {
            if let Some(path_str) = path.to_str() {
                open_video(path_str)
            } else {
                Err(MyError::Application(format!(
                    "{error}: {path:?}",
                    error = ERROR_INVALID_PATH,
                    path = path
                )))
            }
        }
        Some("gif") => open_gif(path),
        _ => {
            // Unknown extension
            // Try webcam
            if let Some(path_str) = path.to_str() {
                if path_str.contains("/dev/video") {
                    open_video(path_str)
                } else {
                    Err(MyError::Application(format!(
                        "Unable to identify media type for:{path_str}"
                    )))
                }
            } else {
                Err(MyError::Application(ERROR_UNSUPPORTED_FORMAT.to_string()))
            }
        }
    }
}
