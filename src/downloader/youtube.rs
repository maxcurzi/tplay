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

/// Direct URLs for a YouTube video, as returned by `yt-dlp -g`.
///
/// YouTube rarely offers pre-muxed formats above 360p, so the video and audio
/// tracks usually live at separate URLs. FFmpeg decodes `video` while MPV plays
/// `audio`, so there is no need to download and mux anything up front.
pub struct StreamUrls {
    /// URL of the video track (or of a muxed stream, when one is available).
    pub video: String,
    /// URL of the separate audio track, if the selected format is not muxed.
    pub audio: Option<String>,
}

/// Format selection for streaming playback.
///
/// Prefers a 720p H.264 track (cheapest to decode, and far more resolution than
/// a terminal can show) and falls back progressively so that videos without a
/// matching format still stream instead of being downloaded in full.
const STREAM_FORMAT: &str =
    "bv*[height<=?720][vcodec^=avc1]+ba/b[height<=?720]/bv*[height<=?720]+ba/bv*+ba/b";

/// Extracts direct streaming URLs from a YouTube video using `yt-dlp -g`.
///
/// Returns URLs that can be passed straight to FFmpeg/MPV without downloading
/// the entire video first.
///
/// # Arguments
///
/// * `url` - The YouTube URL.
/// * `browser` - The browser to use for cookie extraction.
///
/// # Returns
///
/// * `Ok(StreamUrls)` - The direct streaming URLs.
/// * `Err(MyError)` - An error if URL extraction fails.
pub fn get_streaming_url(url: &str, browser: &str) -> Result<StreamUrls, MyError> {
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
        .arg(STREAM_FORMAT)
        .arg("--cookies-from-browser")
        .arg(browser)
        .arg(url)
        .output()
        .map_err(|e| MyError::Application(format!("Failed to run yt-dlp: {}", e)))?;

    if !output.status.success() {
        return Err(MyError::Application(format!(
            "yt-dlp failed to extract URL: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // With a `video+audio` selection yt-dlp prints the video URL first, then
    // the audio URL. A muxed selection prints a single line.
    let mut urls = stdout.lines().map(str::trim).filter(|l| !l.is_empty());

    match urls.next() {
        Some(video) => Ok(StreamUrls {
            video: video.to_string(),
            audio: urls.next().map(str::to_string),
        }),
        None => Err(MyError::Application(
            "yt-dlp returned empty URL".to_string(),
        )),
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
