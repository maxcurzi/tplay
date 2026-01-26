//! This is the high level module for the audio player, it simply provides a
//! basic structure that contains the audio player instance (depending on which
//! audio backend is used). It also defines a trait AudioPlayerControls, which
//! serves as the interface that audio backends are expected to implement.
use crate::MyError;
use std::time::Duration;

#[cfg(not(feature = "rodio_audio"))]
use super::mpv_player::MpvAudioPlayer as BackendAudioPlayer;

#[cfg(feature = "rodio_audio")]
use super::rodio_player::RodioAudioPlayer as BackendAudioPlayer;

pub struct AudioPlayer {
    pub player: BackendAudioPlayer,
}

impl AudioPlayer {
    pub fn new(input_file: &str) -> Result<Self, MyError> {
        let player = BackendAudioPlayer::new(input_file)?;

        Ok(Self { player })
    }
}

pub trait AudioPlayerControls {
    fn pause(&mut self) -> Result<(), MyError>;
    fn resume(&mut self) -> Result<(), MyError>;
    fn stop(&mut self) -> Result<(), MyError>;
    fn toggle_play(&mut self) -> Result<(), MyError>;
    fn mute(&mut self) -> Result<(), MyError>;
    fn unmute(&mut self) -> Result<(), MyError>;
    fn rewind(&mut self) -> Result<(), MyError>;
    fn toggle_mute(&mut self) -> Result<(), MyError>;
    fn seek(&mut self, seconds: f64) -> Result<(), MyError>;
    fn cycle_subtitle(&mut self) -> Result<(), MyError>;
    fn toggle_subtitle(&mut self) -> Result<(), MyError>;
    fn get_subtitle_text(&self) -> Option<String>;
    fn get_position(&self) -> Duration;
    /// Sets the playback speed multiplier (pitch-preserving).
    /// Speed is clamped to range 0.5 to 2.0.
    fn set_speed(&mut self, speed: f64) -> Result<(), MyError>;
    /// Gets the current playback speed multiplier.
    fn get_speed(&self) -> f64;
}
