//! The audio::player module contains the AudioPlayer struct and related functionality for
//! controlling and playing audio files.
//!
//! The AudioPlayer struct is responsible for handling audio playback, including play, pause,
//! mute, unmute, and stop commands.
use mpv::MpvHandler;

use crate::common::errors::MyError;

/// The AudioPlayer struct handles audio playback using the mpv backend.
pub struct AudioPlayer {
    /// The mpv handler responsible for managing the audio playback.
    mpv: MpvHandler,
}

impl AudioPlayer {
    /// Creates a new AudioPlayer instance.
    ///
    /// # Arguments
    ///
    /// * input_path - The path to the audio file to be played.
    ///
    /// # Returns
    ///
    /// A new AudioPlayer instance.
    pub fn new(input_path: &str) -> Result<Self, MyError> {
        let mut mpv_builder = mpv::MpvHandlerBuilder::new().expect("Failed to init MPV builder");
        let _ = mpv_builder.try_hardware_decoding();
        let mut mpv = mpv_builder
            .build()
            .map_err(|err| MyError::Audio(format!("Failed to set no-video property: {:?}", err)))?;
        mpv.set_property("vid", "no")
            .map_err(|err| MyError::Audio(format!("Failed to set no-video property: {:?}", err)))?;
        mpv.command(&["loadfile", input_path])
            .map_err(|err| MyError::Audio(format!("Failed to load audio file: {:?}", err)))?;
        mpv.set_property("pause", false)
            .map_err(|err| MyError::Audio(format!("Failed to set pause property: {:?}", err)))?;
        Ok(Self { mpv })
    }

    /// Pauses the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    pub fn pause(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("pause", true)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Resumes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    pub fn resume(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("pause", false)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Toggles the playback state (play/pause) of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    pub fn toggle_play(&mut self) -> Result<(), MyError> {
        let paused = self
            .mpv
            .get_property::<bool>("pause")
            .map_err(|err| MyError::Audio(format!("{:?}", err)))?;

        if paused {
            self.resume()
        } else {
            self.pause()
        }
    }

    /// Mutes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    pub fn mute(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("mute", true)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Unmutes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    pub fn unmute(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("mute", false)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    pub fn toggle_mute(&mut self) -> Result<(), MyError> {
        let muted = self
            .mpv
            .get_property::<bool>("mute")
            .map_err(|err| MyError::Audio(format!("{:?}", err)))?;
        if muted {
            self.unmute()
        } else {
            self.mute()
        }
    }

    pub fn stop(&mut self) -> Result<(), MyError> {
        self.mpv
            .command(&["stop"])
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }
}
