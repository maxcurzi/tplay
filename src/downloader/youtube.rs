use std::process::Command;
use std::process::Stdio;
use tempfile;
use tempfile::TempPath;

use crate::common::errors::MyError;

pub fn download_video(url: &str) -> Result<TempPath, MyError> {
    // Check that yt-dlp is installed
    if !Command::new("yt-dlp").output().is_ok() {
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
        .tempfile()
        .map_err(|e| MyError::Application(format!("{e:?}")))?;

    let output = Command::new("yt-dlp")
        .arg(url)
        .arg("-o")
        .arg("-")
        .stdout(Stdio::from(
            temp_file
                .as_file()
                .try_clone()
                .map_err(|e| MyError::Application(format!("{e:?}")))?,
        ))
        .output()
        .map_err(|e| MyError::Application(format!("{e:?}")))?;

    if output.status.success() {
        // Flush the buffer to ensure that all the data is written to disk
        temp_file
            .as_file()
            .sync_all()
            .map_err(|e| MyError::Application(format!("{e:?}")))?;

        // Get the path to the temporary file
        let temp_file_path = temp_file.into_temp_path();

        return Ok(temp_file_path);
    } else {
        return Err(MyError::Application(format!(
            "Error downloading video: {:?}",
            output.stderr
        )));
    }
}
