//! This module defines the custom error type `MyError` used throughout the application,
//! as well as various error message constants.

use thiserror::Error;

/// Custom error type used throughout the application.
///
/// This enum provides variants for different categories of errors:
/// * `Application`: General application errors with a string description.
/// * `Terminal`: Terminal-related errors with a string description.
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Application error: {0}")]
    Application(String),

    #[error("Terminal error: {0}")]
    Terminal(String),
}

/// Error message for issues related to opening an image.
pub const ERROR_OPENING_IMAGE: &str = "Error opening image";
/// Error message for issues related to decoding an image.
pub const ERROR_DECODING_IMAGE: &str = "Error decoding image";
/// Error message for issues related to opening a video.
pub const ERROR_OPENING_VIDEO: &str = "Error opening video";
/// Error message for issues related to opening a GIF.
pub const ERROR_OPENING_GIF: &str = "Error opening GIF";
/// Error message for issues related to reading a GIF header.
pub const ERROR_READING_GIF_HEADER: &str = "Cannot read GIF header";
/// Error message for issues related to locking the commands buffer.
pub const ERROR_LOCK_CMD_BUFFER_FAILED: &str = "Failed to lock commands buffer";
/// Error message for issues related to parsing a digit.
pub const ERROR_PARSE_DIGIT_FAILED: &str = "Failed to parse digit";
