//! This module contains the runner utilities for audio playback
//! The Runner struct handles the audio pipeline, processing frames, managing
//! playback state, and controlling the frame rate. It also handles commands for
//! pausing/continuing, and stopping the playback.
use crate::audio;
use crate::audio::player::AudioPlayerControls;
use crate::common::errors::MyError;
use crossbeam_channel::{select, Receiver};

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
}

impl Runner {
    pub fn new(audio_player: audio::player::AudioPlayer, rx_controls: Receiver<Control>) -> Self {
        Self {
            audio_player,
            state: State::Running,
            rx_controls,
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
        while self.state != State::Stopped {
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
                        },
                        Control::MuteUnmute => {
                            self.audio_player.player.toggle_mute()?;
                        },
                        Control::Replay => {
                            self.audio_player.player.rewind()?;
                        },
                        Control::Exit => {
                            self.state = State::Stopped;
                            self.audio_player.player.stop()?;
                        },
                    }
                },
            }
        }
        Ok(())
    }
}
