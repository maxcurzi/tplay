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

/// Extracts a direct streaming URL from a YouTube video using `yt-dlp -g`.
///
/// Returns a muxed (video+audio) streaming URL that can be passed directly to
/// ffmpeg or MPV without downloading the entire file first.
///
/// # Arguments
///
/// * `url` - The YouTube URL.
/// * `browser` - The browser to use for cookie extraction.
///
/// # Returns
///
/// * `Ok(String)` - The direct streaming URL.
/// * `Err(MyError)` - An error if URL extraction fails.
pub fn get_streaming_url(url: &str, browser: &str) -> Result<String, MyError> {
    if Command::new("yt-dlp").output().is_err() {
        return Err(MyError::Application(
            "yt-dlp is not installed.
To view YouTube videos Please install it and try again.
See https://github.com/yt-dlp/yt-dlp/wiki/Installation"
                .to_string(),
        ));
    };

    let output = Command::new("yt-dlp")
        .arg("-g")
        .arg("-f")
        .arg("best")
        .arg("--cookies-from-browser")
        .arg(browser)
        .arg(url)
        .output()
        .map_err(|e| MyError::Application(format!("Failed to run yt-dlp: {}", e)))?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            Err(MyError::Application(
                "yt-dlp returned empty URL".to_string(),
            ))
        } else {
            // yt-dlp may return multiple lines if separate video/audio streams;
            // take the first line (the video+audio muxed URL with -f best)
            let first_url = url.lines().next().unwrap_or(&url).to_string();
            Ok(first_url)
        }
    } else {
        Err(MyError::Application(format!(
            "yt-dlp failed to extract URL: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

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
pub fn download_video(url: &str, browser: &str) -> Result<TempPath, MyError> {
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
        .arg("--cookies-from-browser") // Required by youtube
        .arg(browser) // from cli now --browser <BROWSER>
        // Supported browsers are: brave, chrome, chromium, edge, firefox, opera, safari, vivaldi, whale
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
