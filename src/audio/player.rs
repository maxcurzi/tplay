//! This is the high level module for the audio player, it simply provides a
//! basic structure that contains the audio player instance (depending on which
//! audio backend is used). It also defines a trait AudioPlayerControls, which
//! serves as the interface that audio backends are expected to implement.
use crate::MyError;

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
}
