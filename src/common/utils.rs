use image::{DynamicImage, ImageBuffer};
use num::{Rational64, ToPrimitive};
use opencv::{imgproc, prelude::*};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::str::FromStr;

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
pub fn extract_fps(video_path: &str) -> Option<f64> {
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
pub fn mat_to_dynamic_image(mat: &Mat) -> Option<DynamicImage> {
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
