//! High level audio player control based on rodio
use crate::audio::player::AudioPlayerControls;
use crate::audio::utils::extract_audio;
use crate::common::errors::MyError;
use rodio;
use std::io::{Cursor, Read};
use std::time::Duration;

/// The AudioPlayer struct handles audio playback using the rodio backend.
pub struct RodioAudioPlayer {
    sink: rodio::Sink,
    _stream: rodio::OutputStream,
    #[allow(dead_code)]
    stream_handle: rodio::OutputStreamHandle,
    content: Vec<u8>,
    /// Volume level (for mute/unmute tracking)
    volume: f32,
    /// Current playback speed multiplier (1.0 = normal speed).
    /// Note: Rodio's speed control does affect pitch. For pitch-preserving
    /// playback, use the MPV backend instead.
    current_speed: f64,
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
        
        // Read audio content into memory
        let file = std::fs::File::open(audio_track.path())
            .map_err(|err| MyError::Audio(format!("Failed to open audio file: {:?}", err)))?;
        let mut buf = std::io::BufReader::new(file);
        let mut content = Vec::new();
        buf.read_to_end(&mut content)?;
        
        // Create sink
        let sink = rodio::Sink::try_new(&stream_handle).map_err(|err| {
            MyError::Audio(format!("Failed to create audio sink: {:?}", err))
        })?;
        
        // Create decoder and append to sink
        let cursor = Cursor::new(content.clone());
        let decoder = rodio::decoder::Decoder::new(cursor).map_err(|err| {
            MyError::Audio(format!("Failed to decode audio: {:?}", err))
        })?;
        sink.append(decoder);
        sink.pause(); // Start paused
        
        Ok(Self {
            sink,
            _stream,
            stream_handle,
            content,
            volume: 1.0,
            current_speed: 1.0,
        })
    }

    /// Rebuild the audio pipeline at a given position
    fn rebuild_at_position(&mut self, target_position: Duration) -> Result<(), MyError> {
        let was_paused = self.sink.is_paused();
        
        self.sink.clear();
        
        // Create new decoder
        let cursor = Cursor::new(self.content.clone());
        let decoder = rodio::decoder::Decoder::new(cursor).map_err(|err| {
            MyError::Audio(format!("Failed to decode audio for rebuild: {:?}", err))
        })?;
        
        self.sink.append(decoder);
        
        // Seek to target position
        let _ = self.sink.try_seek(target_position);
        
        // Restore volume
        self.sink.set_volume(self.volume);
        
        // Restore pause state
        if was_paused {
            self.sink.pause();
        } else {
            self.sink.play();
        }
        
        Ok(())
    }
}

impl AudioPlayerControls for RodioAudioPlayer {
    /// Pauses the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn pause(&mut self) -> Result<(), MyError> {
        self.sink.pause();
        Ok(())
    }

    /// Resumes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn resume(&mut self) -> Result<(), MyError> {
        self.sink.play();
        Ok(())
    }

    /// Rewinds the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn rewind(&mut self) -> Result<(), MyError> {
        self.rebuild_at_position(Duration::ZERO)
    }

    /// Toggles the playback state (play/pause) of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_play(&mut self) -> Result<(), MyError> {
        if self.sink.is_paused() {
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
        self.sink.set_volume(0.0);
        Ok(())
    }

    /// Unmutes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn unmute(&mut self) -> Result<(), MyError> {
        self.sink.set_volume(self.volume);
        Ok(())
    }

    /// Toggles the mute state of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_mute(&mut self) -> Result<(), MyError> {
        if self.sink.volume() == 0.0 {
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
        self.sink.stop();
        Ok(())
    }

    /// Seeks forward or backward by the specified number of seconds.
    /// Note: rodio's try_seek may not work for all audio formats or when audio has finished.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The number of seconds to seek. Positive seeks forward, negative seeks backward.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn seek(&mut self, seconds: f64) -> Result<(), MyError> {
        let current_pos = self.sink.get_pos();
        let target_secs = (current_pos.as_secs_f64() + seconds).max(0.0);
        let target_duration = Duration::from_secs_f64(target_secs);
        
        self.rebuild_at_position(target_duration)
    }

    fn cycle_subtitle(&mut self) -> Result<(), MyError> {
        Ok(())
    }

    fn toggle_subtitle(&mut self) -> Result<(), MyError> {
        Ok(())
    }

    fn get_subtitle_text(&self) -> Option<String> {
        None
    }

    fn get_position(&self) -> Duration {
        self.sink.get_pos()
    }

    /// Sets the playback speed multiplier.
    /// Note: Rodio's speed control affects pitch (chipmunk effect at high speeds).
    /// For pitch-preserving playback, use the MPV backend instead.
    ///
    /// # Arguments
    ///
    /// * `speed` - The speed multiplier (1.0 = normal, 0.5 = half, 2.0 = double).
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn set_speed(&mut self, speed: f64) -> Result<(), MyError> {
        let clamped_speed = speed.clamp(0.5, 2.0);
        self.sink.set_speed(clamped_speed as f32);
        self.current_speed = clamped_speed;
        Ok(())
    }

    /// Gets the current playback speed multiplier.
    ///
    /// # Returns
    ///
    /// The current speed multiplier.
    fn get_speed(&self) -> f64 {
        self.current_speed
    }
}
