use std::{path::PathBuf, process::Command};

use tempfile::NamedTempFile;

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

pub fn has_audio(input_path: &str) -> std::io::Result<bool> {
    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-vn") // Disable video
        .arg("-acodec")
        .arg("mp3") // Use the mp3 codec
        .arg("-y") // Overwrite output file if it exists
        .arg("/dev/null")
        .status()?;

    Ok(status.success())
}
