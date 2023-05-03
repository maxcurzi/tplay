use crate::MyError;

#[cfg(any(feature = "mpv_0_34", feature = "mpv_0_35"))]
use super::mpv_player::mpv_player::MpvAudioPlayer;

#[cfg(not(any(feature = "mpv_0_34", feature = "mpv_0_35")))]
use super::rodio_player::rodio_player::RodioAudioPlayer;

pub struct AudioPlayer {
    #[cfg(any(feature = "mpv_0_34", feature = "mpv_0_35"))]
    pub player: MpvAudioPlayer,
    #[cfg(not(any(feature = "mpv_0_34", feature = "mpv_0_35")))]
    pub player: RodioAudioPlayer,
}

impl AudioPlayer {
    pub fn new(input_file: &str) -> Result<Self, MyError> {
        #[cfg(any(feature = "mpv_0_34", feature = "mpv_0_35"))]
        let player = MpvAudioPlayer::new(input_file)?;

        #[cfg(not(any(feature = "mpv_0_34", feature = "mpv_0_35")))]
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
