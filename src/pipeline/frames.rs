/// FRAME ITERATOR DOCUMENTATION
use crate::common::errors::*;
use crate::downloader::youtube;
use gif;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageBuffer};
use opencv::{imgproc, prelude::*, videoio::VideoCapture};
use std::fs::File;
use std::path::Path;
pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedGif {
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
}
use url::Url;

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
        convert_frame_to_grayscale(&frame)
    } else {
        None
    }
}

fn convert_frame_to_grayscale(frame: &Mat) -> Option<DynamicImage> {
    let mut gray_frame = Mat::default();
    if imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0).is_ok() {
        let rows = gray_frame.rows() as u32;
        let cols = gray_frame.cols() as u32;

        let data = gray_frame.data_typed::<u8>().unwrap_or_default();
        let data_vec = data.to_owned(); // Clone the underlying data
        let img = ImageBuffer::from_raw(cols, rows, data_vec).unwrap_or_default();

        Some(DynamicImage::ImageLuma8(img))
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

fn open_video(path: &Path) -> Result<FrameIterator, MyError> {
    let video = VideoCapture::from_file(
        path.to_str().expect(ERROR_OPENING_VIDEO),
        opencv::videoio::CAP_ANY,
    )
    .map_err(|e| MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_VIDEO)))?;

    if video
        .is_opened()
        .map_err(|e| MyError::Application(format!("{error}: {e:?}", error = ERROR_OPENING_VIDEO)))?
    {
        Ok(FrameIterator::Video(video))
    } else {
        Err(MyError::Application(ERROR_OPENING_VIDEO.to_string()))
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

    // Check if the path is a URL and has a YouTube domain
    if let Ok(url) = Url::parse(path.to_str().unwrap_or("")) {
        if let Some(domain) = url.domain() {
            if domain.ends_with("youtube.com") || domain.ends_with("youtu.be") {
                let video = youtube::download_video(path.to_str().unwrap())?;
                return open_video(&video);
            }
        }
    }

    match ext {
        Some("png") | Some("bmp") | Some("ico") | Some("tif") | Some("tiff") | Some("jpg")
        | Some("jpeg") => open_image(path),
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") | Some("mov") | Some("flv")
        | Some("ogg") => open_video(path),
        Some("gif") => open_gif(path),
        _ => open_video(path), // Unknown extension, try to open as video anyway
    }
}
