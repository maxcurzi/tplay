//! This module contains the runner utilities for audio playback
//! The Runner struct handles the audio pipeline, processing frames, managing
//! playback state, and controlling the frame rate. It also handles commands for
//! pausing/continuing, and stopping the playback.
use crate::audio;
use crate::audio::player::AudioPlayerControls;
use crate::common::errors::MyError;
use crate::common::sync::PlaybackClock;
use crossbeam_channel::{select, Receiver};
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Represents the playback state of the Runner.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    /// The Runner is currently reading and processing new  frames.
    Running,
    /// The Runner does not process new frames, but can update the terminal by
    /// processing the last frame again if charset or dimension change.
    Paused,
    /// The Runner was stopped by a command and will cease processing frames,
    /// and eventually exit.
    Stopped,
}

/// The `Runner` struct handles the audio pipeline playback state, including
/// handling commands for pausing/continuing, and stopping.
pub struct Runner {
    /// The audio player responsible for playing the audio file.
    audio_player: audio::player::AudioPlayer,
    /// The current playback state of the Runner.
    state: State,
    /// The channel used to receive commands for pausing/continuing, and stopping.
    rx_controls: Receiver<Control>,
    subtitle_text: Option<Arc<RwLock<String>>>,
    playback_clock: Option<Arc<PlaybackClock>>,
}

/// Enum representing the different control commands that can be sent to the Runner.
#[derive(Debug, PartialEq)]
pub enum Control {
    /// Command to toggle between pause and continue playback.
    PauseContinue,
    /// Replay the audio
    Replay,
    /// Command to toggle between mute and unmute.
    MuteUnmute,
    /// Command to stop the playback and exit the Runner.
    Exit,
    /// Command to seek forward or backward by the specified number of seconds.
    Seek(f64),
    CycleSubtitle,
    ToggleSubtitle,
    /// Command to adjust playback speed by a relative amount (e.g., +0.1, -0.25).
    AdjustSpeed(f64),
    /// Command to reset playback speed to 1.0x.
    ResetSpeed,
}

impl Runner {
    pub fn new(
        audio_player: audio::player::AudioPlayer,
        rx_controls: Receiver<Control>,
        subtitle_text: Option<Arc<RwLock<String>>>,
        playback_clock: Option<Arc<PlaybackClock>>,
    ) -> Self {
        Self {
            audio_player,
            state: State::Running,
            rx_controls,
            subtitle_text,
            playback_clock,
        }
    }

    /// The main function responsible playing the audio file. It handles the
    /// playback state, including handling commands for pausing/continuing, mute/unmute and
    /// stopping.
    ///
    /// # Arguments
    ///
    /// * `barrier` - A barrier used to synchronize the start of the audio playback.
    ///
    /// # Returns
    ///
    /// An empty Result.
    pub fn run(&mut self, barrier: std::sync::Arc<std::sync::Barrier>) -> Result<(), MyError> {
        barrier.wait();
        self.audio_player.player.resume()?;
        
        if let Some(ref clock) = self.playback_clock {
            clock.set_paused(false);
        }
        
        while self.state != State::Stopped {
            self.update_subtitle();
            self.update_playback_clock();
            
            select! {
                recv(self.rx_controls) -> msg => {
                    match msg.unwrap() {
                        Control::PauseContinue => {
                            self.state = match self.state {
                                State::Running => State::Paused,
                                State::Paused => State::Running,
                                State::Stopped => State::Stopped,
                            };
                            self.audio_player.player.toggle_play()?;
                            if let Some(ref clock) = self.playback_clock {
                                clock.set_paused(self.state == State::Paused);
                            }
                        },
                        Control::MuteUnmute => {
                            self.audio_player.player.toggle_mute()?;
                        },
                        Control::Replay => {
                            self.audio_player.player.rewind()?;
                            if let Some(ref clock) = self.playback_clock {
                                clock.set_position(Duration::ZERO);
                            }
                        },
                        Control::Exit => {
                            self.state = State::Stopped;
                            self.audio_player.player.stop()?;
                        },
                        Control::Seek(seconds) => {
                            // Best-effort seek; may not work for all audio backends/formats
                            let _ = self.audio_player.player.seek(seconds);
                            self.update_playback_clock();
                        },
                        Control::CycleSubtitle => {
                            let _ = self.audio_player.player.cycle_subtitle();
                        },
                        Control::ToggleSubtitle => {
                            let _ = self.audio_player.player.toggle_subtitle();
                        },
                        Control::AdjustSpeed(delta) => {
                            let current = self.audio_player.player.get_speed();
                            let new_speed = (current + delta).clamp(0.5, 2.0);
                            if self.audio_player.player.set_speed(new_speed).is_ok() {
                                if let Some(ref clock) = self.playback_clock {
                                    clock.set_speed(new_speed as f32);
                                }
                            }
                        },
                        Control::ResetSpeed => {
                            if self.audio_player.player.set_speed(1.0).is_ok() {
                                if let Some(ref clock) = self.playback_clock {
                                    clock.set_speed(1.0);
                                }
                            }
                        },
                    }
                },
                default(Duration::from_millis(16)) => {}
            }
        }
        Ok(())
    }

    fn update_playback_clock(&self) {
        if let Some(ref clock) = self.playback_clock {
            let pos = self.audio_player.player.get_position();
            clock.set_position(pos);
        }
    }

    fn update_subtitle(&mut self) {
        if let Some(ref subtitle_lock) = self.subtitle_text {
            if let Some(text) = self.audio_player.player.get_subtitle_text() {
                if let Ok(mut subtitle) = subtitle_lock.write() {
                    *subtitle = text;
                }
            } else if let Ok(mut subtitle) = subtitle_lock.write() {
                subtitle.clear();
            }
        }
    }
}
