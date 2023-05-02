//! This module provides a function to download a video from a given URL.
//!
//! The main function `download_video` uses the `yt-dlp` tool to download a video
//! from a given URL and stores it in a temporary file.
//! The function returns a temporary file path to the downloaded video.
//! The temporary file is deleted when the file is closed.
//! The temporary file is created in a temporary directory (OS dependent).
use crate::common::errors::MyError;
use std::process::{Command, Stdio};
use tempfile::{self, TempPath};

/// Downloads a video from the given URL using `yt-dlp` and saves it to a temporary file.
///
/// # Arguments
///
/// * `url` - The URL of the video to download.
///
/// # Returns
///
/// * `Ok(TempPath)` - The path to the downloaded temporary video file.
/// * `Err(MyError)` - An error if the video download fails or if `yt-dlp` is not installed.
///
/// # Errors
///
/// This function can return an error in the following situations:
///
/// * `yt-dlp` is not installed on the system.
/// * The video download fails for any reason.
/// * There is an issue with creating or writing to the temporary file.
pub fn download_video(url: &str) -> Result<TempPath, MyError> {
    // Check that yt-dlp is installed
    if Command::new("yt-dlp").output().is_err() {
        return Err(MyError::Application(
            "yt-dlp is not installed.
To view YouTube videos Please install it and try again.
See https://github.com/yt-dlp/yt-dlp/wiki/Installation"
                .to_string(),
        ));
    };
    // Create a temporary file in the current working directory with the prefix "my_temp_file_" and the suffix ".mp4"
    let temp_file = tempfile::Builder::new()
        .prefix("my_temp_file_")
        .suffix(".webm")
        .tempfile()?;

    let mut cmd = Command::new("yt-dlp");
    cmd.arg(url)
        .arg("-o")
        .arg("-")
        .stdout(Stdio::from(temp_file.as_file().try_clone()?));

    let child = cmd
        .spawn()
        .map_err(|e| MyError::Application(e.to_string()))?;

    let output = child
        .wait_with_output()
        .map_err(|e| MyError::Application(e.to_string()))?;

    if output.status.success() {
        // Flush the buffer to ensure that all the data is written to disk
        temp_file
            .as_file()
            .sync_all()
            .map_err(|e| MyError::Application(e.to_string()))?;

        // Get the path to the temporary file
        let temp_file_path = temp_file.into_temp_path();

        Ok(temp_file_path)
    } else {
        Err(MyError::Application(format!(
            "Error downloading video: {:?}",
            output.stderr
        )))
    }
}
