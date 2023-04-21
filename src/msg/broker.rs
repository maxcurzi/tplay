use crossbeam_channel::{select, Receiver, Sender};

use crate::{audio::runner::Control as AudioControl, pipeline::runner::Control as PipelineControl};

/// Read from terminal and send to pipeline and audio
pub struct MessageBroker {
    rx_channel_terminal: Receiver<PipelineControl>,
    tx_channel_pipeline: Option<Sender<PipelineControl>>,
    tx_channel_audio: Option<Sender<AudioControl>>,
}

impl MessageBroker {
    pub fn new(
        rx_channel_terminal: Receiver<PipelineControl>,
        tx_channel_pipeline: Option<Sender<PipelineControl>>,
        tx_channel_audio: Option<Sender<AudioControl>>,
    ) -> Self {
        Self {
            rx_channel_terminal,
            tx_channel_pipeline,
            tx_channel_audio,
        }
    }

    pub fn run(&mut self) {
        let mut running = true;
        while running || !self.rx_channel_terminal.is_empty() {
            select! {
                recv(self.rx_channel_terminal) -> msg => {
                    match msg {
                        Ok(PipelineControl::Exit) => {
                            running = false;
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::Exit);
                            }
                            if let Some(tx) = &self.tx_channel_audio{
                                let _ = tx.send(AudioControl::Exit);
                            }
                        }
                        Ok(PipelineControl::PauseContinue) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::PauseContinue);
                            }
                            if let Some(tx) = &self.tx_channel_audio {
                                let _ = tx.send(AudioControl::PauseContinue);
                            }
                        }
                        Ok(PipelineControl::Resize(width, height)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::Resize(width, height));
                            }
                        }
                        Ok(PipelineControl::SetCharMap(char_map)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::SetCharMap(char_map));
                            }
                        }
                        Ok(PipelineControl::SetGrayscale(grayscale)) => {
                            if let Some(tx) = &self.tx_channel_pipeline {
                                let _ = tx.send(PipelineControl::SetGrayscale(grayscale));
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
        }
    }
}
