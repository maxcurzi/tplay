// use crate::common::errors::MyError;

/// Pipeline stage for converting images to ASCII art.
use image::{DynamicImage, GrayImage};

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

    // Converts the given image to grayscale and scales it according to the scale stored in this ImagePipeline.
    pub fn process(&self, input: &DynamicImage) -> GrayImage {
        input
            .grayscale()
            .resize_exact(
                self.target_resolution.0,
                self.target_resolution.1,
                image::imageops::FilterType::Nearest,
            )
            .into_luma8()
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

    #[test]
    fn test_new() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        assert_eq!(image.target_resolution, (120, 80));
        assert_eq!(image.char_lookup, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_process() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        let input = image::open("assets/Lenna.png").unwrap();
        let output = image.process(&input);
        assert_eq!(output.width(), 120);
        assert_eq!(output.height(), 80);
    }

    #[test]
    fn test_to_ascii_ext() {
        let image = ImagePipeline::new((120, 80), SHORT_EXT.chars().collect());
        let input = image::open("assets/Lenna.png").unwrap();
        let output = image.to_ascii(&image.process(&input));
        assert_eq!(output.chars().count(), 120 * 80 + 79 * 2); // Resolution + newlines
    }

    #[test]
    fn test_to_ascii() {
        let image = ImagePipeline::new((120, 80), vec!['a', 'b', 'c']);
        let input = image::open("assets/Lenna.png").unwrap();
        let output = image.to_ascii(&image.process(&input));
        assert_eq!(output.len(), 120 * 80 + 79 * 2); // Resolution + newlines
    }
}
