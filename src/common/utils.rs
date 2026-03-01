use num::{Rational64, ToPrimitive};
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
