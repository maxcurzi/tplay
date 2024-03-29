//! High level audio player control based on rodio
use crate::audio::{player::AudioPlayerControls, utils::extract_audio};
use crate::common::errors::MyError;
use rodio;
use std::io::{BufReader, Cursor, Read, Seek};

/// The AudioPlayer struct handles audio playback using the rodio backend.
pub struct RodioAudioPlayer {
    /// The sink responsible for managing the audio playback.
    player: rodio::Sink,
    /// Keep OutputStream alive
    _stream: rodio::OutputStream,
    /// Store content for rewind/replay
    content: Vec<u8>,
}

impl RodioAudioPlayer {
    /// Creates a new AudioPlayer instance.
    ///
    /// # Arguments
    ///
    /// * input_path - The path to the audio file to be played.
    ///
    /// # Returns
    ///
    /// A new AudioPlayer instance.
    pub(crate) fn new(input_path: &str) -> Result<Self, MyError> {
        let (_stream, stream_handle) = rodio::OutputStream::try_default().map_err(|err| {
            MyError::Audio(format!("Failed to initialize audio stream: {:?}", err))
        })?;
        let audio_track = extract_audio(input_path)?;
        // Play audio with rodio
        let file = std::fs::File::open(audio_track.path())
            .map_err(|err| MyError::Audio(format!("Failed to open audio file: {:?}", err)))?;
        let mut buf = BufReader::new(file);
        let mut content = Vec::new();
        buf.by_ref().read_to_end(&mut content)?;
        buf.rewind()?;
        let player: rodio::Sink = stream_handle
            .play_once(buf)
            .map_err(|err| MyError::Audio(format!("Failed to start playback: {:?}", err)))?;
        Ok(Self {
            player,
            _stream,
            content,
        })
    }
}

impl AudioPlayerControls for RodioAudioPlayer {
    /// Pauses the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn pause(&mut self) -> Result<(), MyError> {
        self.player.pause();
        Ok(())
    }

    /// Resumes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn resume(&mut self) -> Result<(), MyError> {
        self.player.play();
        Ok(())
    }

    /// Rewinds the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn rewind(&mut self) -> Result<(), MyError> {
        self.player.clear();
        let input = Cursor::new(self.content.clone());
        let input = rodio::decoder::Decoder::new(input).map_err(|err| {
            MyError::Audio(format!(
                "Could not set decoder on rewind content: {:?}",
                err
            ))
        })?;
        self.player.append(input);
        self.player.play();
        Ok(())
    }

    /// Toggles the playback state (play/pause) of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_play(&mut self) -> Result<(), MyError> {
        if self.player.is_paused() {
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
    fn mute(&mut self) -> Result<(), MyError> {
        self.player.set_volume(0.0);
        Ok(())
    }

    /// Unmutes the audio playback.

    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn unmute(&mut self) -> Result<(), MyError> {
        self.player.set_volume(1.0);
        Ok(())
    }

    /// Toggles the mute state of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_mute(&mut self) -> Result<(), MyError> {
        if self.player.volume() == 0.0 {
            self.unmute()
        } else {
            self.mute()
        }
    }

    /// Stops the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn stop(&mut self) -> Result<(), MyError> {
        self.player.stop();
        Ok(())
    }
}
