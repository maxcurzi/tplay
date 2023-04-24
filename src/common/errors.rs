//! This module defines the custom error type `MyError` used throughout the application,
//! as well as various error message constants.
use std::io;
use thiserror::Error;

/// Custom error type used throughout the application.
///
/// This enum provides variants for different categories of errors:
/// * `Application`: General application errors with a string description.
/// * `Terminal`: Terminal-related errors with a string description.
/// * `Pipeline`: Image pipeline-related errors with a string description.
/// * `Audio`: Audio-related errors with a string description.
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Application error: {0}")]
    Application(String),

    #[error("Image pipeline error: {0}")]
    Pipeline(String),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("Audio error: {0}")]
    Audio(String),
}

impl From<MyError> for io::Error {
    fn from(error: MyError) -> Self {
        io::Error::new(io::ErrorKind::Other, error.to_string())
    }
}

impl From<io::Error> for MyError {
    fn from(error: io::Error) -> Self {
        MyError::Application(format!("{error}"))
    }
}

impl From<opencv::Error> for MyError {
    fn from(error: opencv::Error) -> Self {
        MyError::Application(format!("{error}"))
    }
}

/// Error message for issues related to decoding an image.
pub const ERROR_DECODING_IMAGE: &str = "Error decoding image";
/// Error message for issues related to opening a video.
pub const ERROR_OPENING_VIDEO: &str = "Error opening video";
/// Error message for issues related to opening a GIF.
pub const ERROR_OPENING_GIF: &str = "Error opening GIF";
/// Error message for issues related to reading a GIF header.
pub const ERROR_READING_GIF_HEADER: &str = "Cannot read GIF header";
/// Error message for issues related to parsing a digit.
pub const ERROR_PARSE_DIGIT_FAILED: &str = "Failed to parse digit";
/// Error message for issue related to channel communication.
pub const ERROR_CHANNEL: &str = "Error during channel communication";
/// Error message for issues related to data processing.
pub const ERROR_DATA: &str = "Data error";
/// Error message for issues related to resizing an image.
pub const ERROR_RESIZE: &str = "Image resizing error";
