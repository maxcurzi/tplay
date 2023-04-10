use gif;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use opencv::{imgproc, prelude::*, videoio::VideoCapture};
use std::fs::File;
use std::path::Path;
use std::slice;

pub enum FrameIterator {
    Image(Option<DynamicImage>),
    Video(VideoCapture),
    AnimatedGif {
        // decoder: gif::Decoder<File>,
        frames: Vec<DynamicImage>,
        current_frame: usize,
    },
    Webcam(VideoCapture),
}

impl Iterator for FrameIterator {
    type Item = DynamicImage;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FrameIterator::Image(ref mut img) => img.take(),
            FrameIterator::Video(ref mut video) | FrameIterator::Webcam(ref mut video) => {
                let mut frame = Mat::default();

                if video.read(&mut frame).unwrap() && !frame.empty() {
                    // Convert the frame from BGR to RGB format
                    let mut rgb_frame = Mat::default();
                    imgproc::cvt_color(&frame, &mut rgb_frame, imgproc::COLOR_BGR2RGB, 0).unwrap();

                    let rows = rgb_frame.rows() as usize;
                    let cols = rgb_frame.cols() as usize;
                    let channels = rgb_frame.channels() as usize;
                    let data_ptr = rgb_frame.data();
                    let data_slice =
                        unsafe { slice::from_raw_parts(data_ptr, rows * cols * channels) };

                    Some(DynamicImage::ImageRgb8(
                        image::RgbImage::from_raw(cols as u32, rows as u32, data_slice.to_vec())
                            .unwrap(),
                    ))
                } else {
                    None
                }
            }
            FrameIterator::AnimatedGif {
                // ref mut decoder,
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

pub fn open_media<P: AsRef<Path>>(path: P) -> Result<FrameIterator, String> {
    let path = path.as_ref();
    let ext = path.extension().and_then(std::ffi::OsStr::to_str);

    match ext {
        Some("png") | Some("jpg") | Some("jpeg") => {
            let img = ImageReader::open(path)
                .map_err(|e| format!("Error opening image: {:?}", e))?
                .decode()
                .map_err(|e| format!("Error decoding image: {:?}", e))?;
            Ok(FrameIterator::Image(Some(img)))
        }
        Some("mp4") | Some("avi") | Some("webm") | Some("mkv") => {
            let video = VideoCapture::from_file(path.to_str().unwrap(), opencv::videoio::CAP_ANY)
                .map_err(|e| format!("Error opening video: {:?}", e))?;

            if video.is_opened().unwrap() {
                Ok(FrameIterator::Video(video))
            } else {
                Err("Could not open the video".to_string())
            }
        }
        Some("gif") => {
            let file = File::open(path).map_err(|e| format!("Error opening GIF: {:?}", e))?;
            let mut options = gif::DecodeOptions::new();
            options.set_color_output(gif::ColorOutput::RGBA);
            // Read the file header
            let mut decoder = options.read_info(file).unwrap();
            // Ok(FrameIterator::AnimatedGif(decoder))
            let mut frames = Vec::new();

            while let Ok(Some(frame)) = decoder.read_next_frame() {
                let buffer = frame.buffer.clone();
                let image = image::RgbaImage::from_raw(
                    decoder.width() as u32,
                    decoder.height() as u32,
                    buffer.to_vec(),
                )
                .unwrap();
                frames.push(DynamicImage::ImageRgba8(image));
            }

            let frame_iterator = FrameIterator::AnimatedGif {
                // decoder,
                frames,
                current_frame: 0,
            };
            Ok(frame_iterator)
        }
        _ => {
            // Unknown extension
            // Try webcam
            if path.to_str().unwrap().contains("/dev/video") {
                let video =
                    VideoCapture::from_file(path.to_str().unwrap(), opencv::videoio::CAP_ANY)
                        .map_err(|e| format!("Error opening video: {:?}", e))?;

                if video.is_opened().unwrap() {
                    Ok(FrameIterator::Webcam(video))
                } else {
                    Err("Could not open the video".to_string())
                }
            } else {
                Err("Unsupported file format".to_string())
            }
        }
    }
}
