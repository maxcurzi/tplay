//! Message broker for the terminal, pipeline and audio.
//!
//! The broker is responsible for handling the communication between the terminal, pipeline and
//! audio threads. It receives commands from the terminal and forwards them to the pipeline and
//! audio threads, and receives commands from the pipeline and audio threads and forwards them to
//! the terminal thread.
use crate::{
    audio::runner::Control as AudioControl, common::errors::MyError,
    pipeline::runner::Control as PipelineControl,
};
use crossbeam_channel::{select, Receiver, Sender};

/// Enum representing the different control commands that can be sent to the Runner.
#[derive(Debug, PartialEq)]
pub enum Control {
    /// Command to toggle between pause and continue playback.
    PauseContinue,
    /// Command to stop the playback and exit the Runner.
    Exit,
    /// Command to toggle between mute and unmute.
    MuteUnmute,
    /// Command to set the character map used by the image pipeline.
    /// The argument represents the index of the desired character map.
    SetCharMap(u32),
    /// Command to resize the target resolution of the image pipeline.
    /// The arguments represent the new target width and height, respectively.
    Resize(u16, u16),
    /// Command to set grayscale mode. We always extract rgb+grayscale from image, the terminal is
    /// responsible for the correct render mode.
    SetGrayscale(bool),
}

type BrokerControl = Control;

/// Read from terminal and send to pipeline and audio
pub struct MessageBroker {
    rx_channel_terminal: Receiver<BrokerControl>,
    tx_channel_pipeline: Option<Sender<PipelineControl>>,
    tx_channel_audio: Option<Sender<AudioControl>>,
}

impl MessageBroker {
    pub fn new(
        rx_channel_terminal: Receiver<BrokerControl>,
        tx_channel_pipeline: Option<Sender<PipelineControl>>,
        tx_channel_audio: Option<Sender<AudioControl>>,
    ) -> Self {
        Self {
            rx_channel_terminal,
            tx_channel_pipeline,
            tx_channel_audio,
        }
    }

    /// The main function responsible for handling the communication between the terminal, pipeline
    /// and audio threads.
    ///
    /// It receives commands from the terminal and forwards them to the pipeline and audio threads,
    /// and receives commands from the pipeline and audio threads and forwards them to the terminal
    /// thread.
    ///
    /// # Arguments
    ///
    /// * `barrier` - A barrier used to synchronize the start of the audio playback.
    ///
    /// # Returns
    ///
    /// * `Result<(), MyError>` - A result indicating whether the function succeeded or failed.
    pub fn run(&mut self, barrier: std::sync::Arc<std::sync::Barrier>) -> Result<(), MyError> {
        barrier.wait();
        let mut running = true;
        while running || !self.rx_channel_terminal.is_empty() {
            select! {
                recv(self.rx_channel_terminal) -> msg => {
                    match msg {
                        Ok(BrokerControl::Exit) => {
                            running = false;
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::Exit);
                            }
                            if let Some(tx) = &self.tx_channel_audio{
                                let _ = tx.send(AudioControl::Exit);
                            }
                        }
                        Ok(BrokerControl::PauseContinue) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::PauseContinue);
                            }
                            if let Some(tx) = &self.tx_channel_audio {
                                let _ = tx.send(AudioControl::PauseContinue);
                            }
                        }
                        Ok(BrokerControl::Resize(width, height)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::Resize(width, height));
                            }
                        }
                        Ok(BrokerControl::SetCharMap(char_map)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::SetCharMap(char_map));
                            }
                        }
                        Ok(BrokerControl::SetGrayscale(grayscale)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::SetGrayscale(grayscale));
                            }
                        }
                        Ok(BrokerControl::MuteUnmute) => {
                            if let Some(tx) = &self.tx_channel_audio {
                                let _ = tx.send(AudioControl::MuteUnmute);
                            }
                        }
                        Err(_) => {
                            // eprintln!("Error: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
