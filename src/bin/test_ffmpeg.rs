use ffmpeg_next::codec::decoder::Video;
use ffmpeg_next::codec::Context;
use ffmpeg_next::format::context::input::PacketIter;
// use ffmpeg_next::error::Result;
use ffmpeg_next::format::context::Input;
use ffmpeg_next::format::{input, Pixel};
use ffmpeg_next::packet;
use ffmpeg_next::util::frame::video::Video as VideoFrame;
use image::{DynamicImage, RgbImage, RgbaImage};
use std::path::Path;

pub struct FrameIterator {
    video_stream_index: usize,
    decoder: Video,
    // packets: PacketIter,
    input_context: Input,
}

impl FrameIterator {
    pub fn new(video_path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        ffmpeg_next::init()?;
        let input_context = input(&video_path)?;

        let video_stream = input_context
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(ffmpeg_next::Error::StreamNotFound)?;

        let video_stream_index = video_stream.index();

        let decoder = video_stream.codec().decoder().video()?;

        // let packets = input_context.packets();

        Ok(FrameIterator {
            video_stream_index,
            decoder,
            // packets,
            input_context,
        })
    }
}

impl Iterator for FrameIterator {
    type Item = Result<VideoFrame, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut decoded_frame = VideoFrame::empty();
        while let Some((stream, packet)) = self.input_context.packets().next() {
            if stream.index() == self.video_stream_index {
                match self.decoder.send_packet(&packet) {
                    Ok(_) => {}
                    Err(e) => return Some(Err(Box::new(e))),
                }

                match self.decoder.receive_frame(&mut decoded_frame) {
                    Ok(_) => return Some(Ok(decoded_frame.clone())),
                    Err(e) => return Some(Err(Box::new(e))),
                }
            }
        }
        None
    }
}

pub fn video_frame_to_dynamic_image(frame: &VideoFrame) -> Result<DynamicImage, Box<dyn Error>> {
    let width = frame.width() as u32;
    let height = frame.height() as u32;

    // if frame.format() != Pixel::RGB8 {
    //     return Err(Box::new(ffmpeg_next::Error::InvalidData));
    // }

    let buffer = frame.data(0);
    let mut rgb_image = RgbImage::new(width, height);

    for (x, y, pixel) in rgb_image.enumerate_pixels_mut() {
        let offset = (y * width + x) as usize;
        *pixel = image::Rgb([
            buffer[offset],
            buffer[offset],
            buffer[offset],
            // buffer[offset + 1],
            // buffer[offset + 2],
            // buffer[offset + 3],
        ]);
    }

    Ok(DynamicImage::ImageRgb8(rgb_image))
}

use image::GenericImageView;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let video_path = "assets/eva.webm";

    let frame_iterator = FrameIterator::new(video_path)?;

    for frame_result in frame_iterator {
        match frame_result {
            Ok(frame) => {
                let dynamic_image = video_frame_to_dynamic_image(&frame)?;
                let (width, height) = dynamic_image.dimensions();
                println!("Frame size: {}x{}", width, height);
            }
            Err(e) => eprintln!("Error while processing a frame: {}", e),
        }
    }

    Ok(())
}
