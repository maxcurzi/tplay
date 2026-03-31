//! High level audio player control based on MPV
use crate::audio::player::AudioPlayerControls;
use crate::common::errors::MyError;
use libmpv::Mpv;
use std::time::Duration;

/// The AudioPlayer struct handles audio playback using the libmpv backend.
pub struct MpvAudioPlayer {
    /// The mpv instance responsible for managing the audio playback.
    mpv: Mpv,
    /// Current playback speed multiplier (1.0 = normal speed).
    current_speed: f64,
}

impl MpvAudioPlayer {
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
        let mpv = Mpv::new().expect("Failed to init MPV builder");

        mpv.set_property("vid", "no")
            .map_err(|err| MyError::Audio(format!("Failed to set no-video property: {:?}", err)))?;
        mpv.set_property("audio-display", "no").map_err(|err| {
            MyError::Audio(format!(
                "Failed to set no-audio-display property: {:?}",
                err
            ))
        })?;
        // Enable pitch-preserving time-stretching (scaletempo2 is MPV's default)
        mpv.set_property("audio-pitch-correction", "yes").map_err(|err| {
            MyError::Audio(format!(
                "Failed to set audio-pitch-correction property: {:?}",
                err
            ))
        })?;

        mpv.command("loadfile", &[input_path])
            .map_err(|err| MyError::Audio(format!("Failed to load audio file: {:?}", err)))?;
        mpv.set_property("pause", true)
            .map_err(|err| MyError::Audio(format!("Failed to set pause property: {:?}", err)))?;

        Ok(Self { mpv, current_speed: 1.0 })
    }
}
impl AudioPlayerControls for MpvAudioPlayer {
    /// Pauses the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn pause(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("pause", true)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Resumes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn resume(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("pause", false)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Toggles the playback state (play/pause) of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_play(&mut self) -> Result<(), MyError> {
        let paused = self
            .mpv
            .get_property("pause")
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
    fn mute(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("mute", true)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Unmutes the audio playback.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn unmute(&mut self) -> Result<(), MyError> {
        self.mpv
            .set_property("mute", false)
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    /// Toggles the mute state of the audio.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_mute(&mut self) -> Result<(), MyError> {
        let muted = self
            .mpv
            .get_property("mute")
            .map_err(|err| MyError::Audio(format!("{:?}", err)))?;

        if muted {
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
        self.mpv
            .command("stop", &["false"])
            .map_err(|err| MyError::Audio(format!("{:?}", err)))
    }

    fn rewind(&mut self) -> Result<(), MyError> {
        // TODO
        Err(MyError::Audio(
            "Rewind feature not implemented for MPV audio player".to_string(),
        ))
    }

    /// Seeks forward or backward by the specified number of seconds.
    ///
    /// # Arguments
    ///
    /// * `seconds` - The number of seconds to seek. Positive seeks forward, negative seeks backward.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn seek(&mut self, seconds: f64) -> Result<(), MyError> {
        let seek_arg = format!("{}", seconds);
        self.mpv
            .command("seek", &[&seek_arg, "relative"])
            .map_err(|err| MyError::Audio(format!("Seek failed: {:?}", err)))
    }

    fn seek_absolute(&mut self, seconds: f64) -> Result<(), MyError> {
        let seek_arg = format!("{}", seconds);
        self.mpv
            .command("seek", &[&seek_arg, "absolute"])
            .map_err(|err| MyError::Audio(format!("Seek absolute failed: {:?}", err)))
    }

    fn seek_percent(&mut self, pct: f64) -> Result<(), MyError> {
        let percent = (pct * 100.0).clamp(0.0, 100.0);
        let seek_arg = format!("{}", percent);
        self.mpv
            .command("seek", &[&seek_arg, "absolute-percent"])
            .map_err(|err| MyError::Audio(format!("Seek percent failed: {:?}", err)))
    }

    /// Cycles through available subtitle tracks.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn cycle_subtitle(&mut self) -> Result<(), MyError> {
        self.mpv
            .command("cycle", &["sub"])
            .map_err(|err| MyError::Audio(format!("Cycle subtitle failed: {:?}", err)))
    }

    /// Toggles subtitle visibility on/off.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an `MyError::Audio` error.
    fn toggle_subtitle(&mut self) -> Result<(), MyError> {
        let visible: bool = self.mpv.get_property("sub-visibility").unwrap_or(true);
        self.mpv
            .set_property("sub-visibility", !visible)
            .map_err(|err| MyError::Audio(format!("Toggle subtitle failed: {:?}", err)))
    }

    /// Sets the playback speed multiplier (pitch-preserving via scaletempo2).
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
        self.mpv
            .set_property("speed", clamped_speed)
            .map_err(|err| MyError::Audio(format!("Set speed failed: {:?}", err)))?;
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

    fn get_subtitle_text(&self) -> Option<String> {
        self.mpv.get_property::<String>("sub-text").ok()
    }

    fn get_position(&self) -> Duration {
        let pos: f64 = self.mpv.get_property("time-pos").unwrap_or(0.0);
        Duration::from_secs_f64(pos.max(0.0))
    }
}
