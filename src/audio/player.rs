use crate::MyError;

#[cfg(not(feature = "rodio_audio"))]
use super::mpv_player::mpv_player::MpvAudioPlayer;

#[cfg(feature = "rodio_audio")]
use super::rodio_player::rodio_player::RodioAudioPlayer;

pub struct AudioPlayer {
    #[cfg(not(feature = "rodio_audio"))]
    pub player: MpvAudioPlayer,

    #[cfg(feature = "rodio_audio")]
    pub player: RodioAudioPlayer,
}

impl AudioPlayer {
    pub fn new(input_file: &str) -> Result<Self, MyError> {
        #[cfg(not(feature = "rodio_audio"))]
        let player = MpvAudioPlayer::new(input_file)?;

        #[cfg(feature = "rodio_audio")]
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
