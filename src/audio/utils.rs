//! This module contains utilities for working with audio files. It uses the
//! `ffmpeg` command line tool to extract the audio from the video file, and
//! convert it to mp3 format.
//! The `has_audio` function uses the `ffprobe` command line tool to check if
//! the video file contains an audio stream.
//! The `extract_audio` function uses the `ffmpeg` command line tool to extract
//! the audio stream from the video file, and convert it to mp3 format.
use crate::common::errors::MyError;
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

#[allow(dead_code)]
pub fn extract_audio(input_path: &str) -> std::io::Result<NamedTempFile> {
    let output_temp = tempfile::Builder::new()
        .prefix("my_temp_file_")
        .suffix(".mp3")
        .tempfile()?;
    let output_path: PathBuf = output_temp.path().to_path_buf();

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
        Ok(output_temp)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to extract audio track",
        ))
    }
}

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
