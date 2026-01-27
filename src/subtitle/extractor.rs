//! Subtitle track extraction using ffprobe and ffmpeg.
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Debug, Clone)]
pub struct SubtitleTrack {
    #[allow(dead_code)]
    pub index: u32,
    #[allow(dead_code)]
    pub language: Option<String>,
    pub path: PathBuf,
    pub _temp_file: std::sync::Arc<NamedTempFile>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<StreamInfo>,
}

#[derive(Deserialize)]
struct StreamInfo {
    index: u32,
    #[serde(default)]
    tags: Option<StreamTags>,
}

#[derive(Deserialize)]
struct StreamTags {
    language: Option<String>,
}

pub fn extract_subtitles(media_path: &Path) -> Vec<SubtitleTrack> {
    let streams = match get_subtitle_streams(media_path) {
        Some(s) => s,
        None => return Vec::new(),
    };

    if streams.is_empty() {
        return Vec::new();
    }

    let handles: Vec<_> = streams
        .into_iter()
        .map(|(index, language)| {
            let path = media_path.to_path_buf();
            std::thread::spawn(move || extract_single_subtitle(&path, index, language))
        })
        .collect();

    handles
        .into_iter()
        .filter_map(|h| h.join().ok().flatten())
        .collect()
}

fn get_subtitle_streams(media_path: &Path) -> Option<Vec<(u32, Option<String>)>> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "s",
            "-show_entries", "stream=index:stream_tags=language",
            "-of", "json",
        ])
        .arg(media_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let probe: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;
    
    Some(
        probe
            .streams
            .into_iter()
            .map(|s| {
                let lang = s.tags.and_then(|t| t.language);
                (s.index, lang)
            })
            .collect(),
    )
}

fn extract_single_subtitle(
    media_path: &Path,
    stream_index: u32,
    language: Option<String>,
) -> Option<SubtitleTrack> {
    let mut temp_file = NamedTempFile::new().ok()?;
    let temp_path = temp_file.path().to_path_buf();

    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-v", "error",
            "-i",
        ])
        .arg(media_path)
        .args([
            "-map", &format!("0:{}", stream_index),
            "-f", "srt",
        ])
        .arg(&temp_path)
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    temp_file.flush().ok()?;

    Some(SubtitleTrack {
        index: stream_index,
        language,
        path: temp_path,
        _temp_file: std::sync::Arc::new(temp_file),
    })
}
