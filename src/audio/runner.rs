use crate::audio;
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

/// The `Runner` struct handles the image pipeline, processing frames, managing
/// playback state, and controlling the frame rate. It also handles commands for
/// pausing/continuing, resizing, and changing character maps during playback.
pub struct Runner {
    ///
    audio_player: audio::player::AudioPlayer,
    /// The current playback state of the Runner.
    state: State,
    /// Barrier
    barrier: std::sync::Arc<std::sync::Barrier>,
    ///
    rx_controls: Receiver<Control>,
}

/// Enum representing the different control commands that can be sent to the Runner.
#[derive(Debug, PartialEq)]
pub enum Control {
    /// Command to toggle between pause and continue playback.
    PauseContinue,
    /// Command to stop the playback and exit the Runner.
    Exit,
}

impl Runner {
    pub fn new(
        audio_player: audio::player::AudioPlayer,
        rx_controls: Receiver<Control>,
        barrier: std::sync::Arc<std::sync::Barrier>,
    ) -> Self {
        Self {
            audio_player,
            state: State::Running,
            barrier,
            rx_controls,
        }
    }

    pub fn run(&mut self) -> Result<(), MyError> {
        self.barrier.wait();
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
                            self.audio_player.toggle();

                        },
                        Control::Exit => {
                            self.state = State::Stopped;
                            self.audio_player.stop();
                        },
                    }
                },
                // default(Duration::from_millis(100)) => {
                //     if self.state == State::Running {
                //         // do something
                //     }
                // },
            }
        }
        Ok(())
    }
}
