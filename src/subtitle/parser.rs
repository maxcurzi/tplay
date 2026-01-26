//! SRT subtitle parser
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::SubtitleTrack;

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
}

pub struct SubtitleManager {
    tracks: Vec<(SubtitleTrack, Vec<SubtitleEntry>)>,
    current_track: usize,
    enabled: bool,
}

impl SubtitleManager {
    pub fn new(tracks: Vec<SubtitleTrack>) -> Self {
        let parsed_tracks: Vec<_> = tracks
            .into_iter()
            .filter_map(|track| {
                let entries = parse_srt_file(&track.path)?;
                Some((track, entries))
            })
            .collect();

        Self {
            tracks: parsed_tracks,
            current_track: 0,
            enabled: false,
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    #[allow(dead_code)]
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    #[allow(dead_code)]
    pub fn current_track_info(&self) -> Option<(usize, Option<&str>)> {
        self.tracks.get(self.current_track).map(|(track, _)| {
            (self.current_track, track.language.as_deref())
        })
    }

    pub fn cycle_track(&mut self) {
        if !self.tracks.is_empty() {
            self.current_track = (self.current_track + 1) % self.tracks.len();
        }
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_subtitle_at(&self, position: Duration) -> Option<&str> {
        if !self.enabled || self.tracks.is_empty() {
            return None;
        }

        let (_, entries) = &self.tracks[self.current_track];
        
        for entry in entries {
            if position >= entry.start && position <= entry.end {
                return Some(&entry.text);
            }
        }
        
        None
    }
}

fn parse_srt_file(path: &Path) -> Option<Vec<SubtitleEntry>> {
    let content = fs::read_to_string(path).ok()?;
    Some(parse_srt(&content))
}

fn parse_srt(content: &str) -> Vec<SubtitleEntry> {
    let mut entries = Vec::new();
    let mut lines = content.lines().peekable();
    
    while lines.peek().is_some() {
        while let Some(line) = lines.peek() {
            if line.trim().parse::<u32>().is_ok() {
                lines.next();
                break;
            }
            lines.next();
        }

        let timing_line = match lines.next() {
            Some(l) => l,
            None => break,
        };

        let (start, end) = match parse_timing_line(timing_line) {
            Some(t) => t,
            None => continue,
        };

        let mut text_lines = Vec::new();
        while let Some(line) = lines.peek() {
            if line.trim().is_empty() {
                lines.next();
                break;
            }
            text_lines.push(*line);
            lines.next();
        }

        let raw_text = text_lines.join(" ");
        let clean_text = strip_formatting(&raw_text);

        if !clean_text.is_empty() {
            entries.push(SubtitleEntry {
                start,
                end,
                text: clean_text,
            });
        }
    }

    entries
}

fn parse_timing_line(line: &str) -> Option<(Duration, Duration)> {
    let parts: Vec<&str> = line.split("-->").collect();
    if parts.len() != 2 {
        return None;
    }

    let start = parse_timestamp(parts[0].trim())?;
    let end = parse_timestamp(parts[1].trim())?;
    Some((start, end))
}

fn parse_timestamp(ts: &str) -> Option<Duration> {
    let ts = ts.split(',').next()?.trim();
    let ts = ts.split('.').next()?.trim();
    
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours: u64 = parts[0].parse().ok()?;
    let minutes: u64 = parts[1].parse().ok()?;
    let seconds: u64 = parts[2].parse().ok()?;

    Some(Duration::from_secs(hours * 3600 + minutes * 60 + seconds))
}
fn strip_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '<' => {
                // Skip until '>'
                while let Some(c) = chars.next() {
                    if c == '>' { break; }
                }
            }
            '{' if chars.peek() == Some(&'\\') => {
                // Skip ASS tags: {\...}
                while let Some(c) = chars.next() {
                    if c == '}' { break; }
                }
            }
            '&' => {
                // Handle HTML entities
                let entity: String = chars.by_ref()
                    .take_while(|&c| c != ';')
                    .collect();
                result.push(match entity.as_str() {
                    "nbsp" => ' ',
                    "lt" => '<',
                    "gt" => '>',
                    "amp" => '&',
                    "quot" => '"',
                    _ => {
                        result.push('&');
                        result.push_str(&entity);
                        continue;
                    }
                });
            }
            _ => result.push(ch),
        }
    }
    
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}
