use crossbeam_channel::{select, Receiver, Sender};

use crate::{
    audio::runner::Control as AudioControl, common::errors::MyError,
    pipeline::runner::Control as PipelineControl,
};

/// Enum representing the different control commands that can be sent to the Runner.
#[derive(Debug, PartialEq)]
pub enum Control {
    /// Command to toggle between pause and continue playback.
    PauseContinue,
    /// Command to stop the playback and exit the Runner.
    Exit,
    MuteUnmute,
    /// Command to set the character map used by the image pipeline.
    /// The argument represents the index of the desired character map.
    SetCharMap(u32),
    /// Command to resize the target resolution of the image pipeline.
    /// The arguments represent the new target width and height, respectively.
    Resize(u16, u16),
    /// (UNUSED) Command to set grayscale mode. We always extract
    /// rgb+grayscale from image, the terminal is responsible for the correct
    /// render mode.
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

    pub fn run(&mut self) -> Result<(), MyError> {
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
                        Err(e) => {
                            // eprintln!("Error: {}", e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
