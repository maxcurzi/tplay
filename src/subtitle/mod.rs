//! Subtitle extraction and parsing for local media files.
//! Uses ffprobe to detect subtitle tracks and ffmpeg to extract them to temporary .srt files.
mod extractor;
mod parser;

pub use extractor::{SubtitleTrack, extract_subtitles};
pub use parser::SubtitleManager;
