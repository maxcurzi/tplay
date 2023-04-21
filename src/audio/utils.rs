use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

use crate::common::errors::MyError;

pub fn extract_audio(input_path: &str) -> std::io::Result<PathBuf> {
    let output_temp = NamedTempFile::new()?;
    let output_path = output_temp.path().with_extension("mp3");

    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-vn") // Disable video
        .arg("-acodec")
        .arg("mp3") // Use the mp3 codec
        .arg("-y") // Overwrite output file if it exists
        .arg(&output_path)
        .status()?;

    if status.success() {
        Ok(output_path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to extract audio track",
        ))
    }
}

// pub fn has_audio(input_path: &str) -> std::io::Result<bool> {
//     let status = Command::new("ffmpeg")
//         .arg("-i")
//         .arg(input_path)
//         .arg("-vn") // Disable video
//         .arg("-acodec")
//         .arg("mp3") // Use the mp3 codec
//         .arg("-y") // Overwrite output file if it exists
//         .arg("/dev/null")
//         .status()?;

//     Ok(status.success())
// }

pub fn has_audio(file_path: &str) -> Result<bool, MyError> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("a") // Select audio streams
        .arg("-show_entries")
        .arg("stream=codec_name")
        .arg("-of")
        .arg("json")
        .arg(file_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;

    let output_str =
        String::from_utf8(output.stdout).map_err(|err| MyError::Application(format!("{err:?}")))?;
    let json_value: Value = serde_json::from_str(&output_str)
        .map_err(|err| MyError::Application(format!("{err:?}")))?;

    Ok(json_value["streams"]
        .as_array()
        .map_or(false, |streams| !streams.is_empty()))
}
