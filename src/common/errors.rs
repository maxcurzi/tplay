use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Application error: {0}")]
    Application(String),

    // #[error("Pipeline error: {0}")]
    // Pipeline(String),

    #[error("Terminal error: {0}")]
    Terminal(String),
}

pub const ERROR_OPENING_IMAGE: &str = "Error opening image";
pub const ERROR_DECODING_IMAGE: &str = "Error decoding image";
pub const ERROR_OPENING_VIDEO: &str = "Error opening video";
pub const ERROR_INVALID_PATH: &str = "Invalid path";
pub const ERROR_OPENING_GIF: &str = "Error opening GIF";
pub const ERROR_READING_GIF_HEADER: &str = "Cannot read GIF header";
pub const ERROR_UNSUPPORTED_FORMAT: &str = "Unsupported file format";
