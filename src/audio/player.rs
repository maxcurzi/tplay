use crate::MyError;

use super::rodio_player::RodioAudioPlayer;

pub struct AudioPlayer {
    pub player: RodioAudioPlayer,
}

impl AudioPlayer {
    pub fn new(input_file: &str) -> Result<Self, MyError> {
        let player = RodioAudioPlayer::new(input_file)?;

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
    fn toggle_mute(&mut self) -> Result<(), MyError>;
}
