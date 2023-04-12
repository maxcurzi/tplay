/// DOCUMENTATION Pipeline stage for converting images to ASCII art.
use fast_image_resize as fr;
use image::{DynamicImage, GrayImage};
use std::num::NonZeroU32;

impl ImagePipeline {
    // Constructs a new ImagePipeline with the given scale, character lookup table, and average window size.
    pub fn new(target_resolution: (u32, u32), char_lookup: Vec<char>) -> Self {
        Self {
            target_resolution,
            char_lookup,
        }
    }
    pub fn set_target_resolution(&mut self, width: u32, height: u32) -> &mut Self {
        self.target_resolution = (width, height);
        self
    }

    // Scale the image according to the scale stored in this ImagePipeline.
    pub fn process(&self, img: &DynamicImage) -> GrayImage {
        let width = NonZeroU32::new(img.width()).unwrap();
        let height = NonZeroU32::new(img.height()).unwrap();
        let src_image = fr::Image::from_vec_u8(
            width,
            height,
            img.to_owned().into_luma8().to_vec(),
            fr::PixelType::U8,
        )
        .unwrap();
        let mut dst_image = fr::Image::new(
            NonZeroU32::new(self.target_resolution.0).unwrap(),
            NonZeroU32::new(self.target_resolution.1).unwrap(),
            fr::PixelType::U8,
        );
        let mut dst_view = dst_image.view_mut();

        let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
        resizer.resize(&src_image.view(), &mut dst_view).unwrap();

        let dst_image = dst_image.into_vec();
        GrayImage::from_vec(
            self.target_resolution.0,
            self.target_resolution.1,
            dst_image,
        )
        .unwrap()
    }

    // Converts the given grayscale image to ASCII art using the character lookup table stored in this ImagePipeline.
    pub fn to_ascii(&self, input: &GrayImage) -> String {
        let (width, height) = (input.width(), input.height());
        let capacity = (width + 1) * height + 1;
        let mut output = String::with_capacity(capacity as usize);

        for y in 0..height {
            output.extend((0..width).map(|x| {
                let lum = input.get_pixel(x, y)[0] as u32;
                let lookup_idx = self.char_lookup.len() * lum as usize / (u8::MAX as usize + 1);
                self.char_lookup[lookup_idx]
            }));
            if y < height - 1 {
                output.push('\r');
                output.push('\n');
            }
        }

        output
    }
}

pub struct ImagePipeline {
    pub target_resolution: (u32, u32),
    pub char_lookup: Vec<char>,
}

#[cfg(test)]
mod tests {
    use crate::pipeline::char_maps::SHORT_EXT;

    use super::*;
    use image::{DynamicImage, ImageError};
    use reqwest;
    use std::io::Cursor;

    fn download_image(url: &str) -> Result<DynamicImage, ImageError> {
        let response = reqwest::blocking::get(url)
            .expect("Failed to download image")
            .bytes()
            .expect("Failed to get image bytes");

        let image_data = Cursor::new(response);
        image::load(image_data, image::ImageFormat::Png)
    }

    #[test]
    fn test_new() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        assert_eq!(image.target_resolution, (120, 80));
        assert_eq!(image.char_lookup, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_process() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        let input = download_image(
            "https://upload.wikimedia.org/wikipedia/en/7/7d/Lenna_%28test_image%29.png",
        )
        .expect("Failed to download image");

        let output = image.process(&input);
        assert_eq!(output.width(), 120);
        assert_eq!(output.height(), 80);
    }

    #[test]
    fn test_to_ascii_ext() {
        let image = ImagePipeline::new((120, 80), SHORT_EXT.chars().collect());
        let input = download_image(
            "https://upload.wikimedia.org/wikipedia/en/7/7d/Lenna_%28test_image%29.png",
        )
        .expect("Failed to download image");
        let output = image.to_ascii(&image.process(&input));
        assert_eq!(output.chars().count(), 120 * 80 + 79 * 2); // Resolution + newlines
    }

    #[test]
    fn test_to_ascii() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        let input = download_image(
            "https://upload.wikimedia.org/wikipedia/en/7/7d/Lenna_%28test_image%29.png",
        )
        .expect("Failed to download image");
        let output = image.to_ascii(&image.process(&input));
        assert_eq!(output.len(), 120 * 80 + 79 * 2); // Resolution + newlines
    }
}
