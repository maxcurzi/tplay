//! Main module for the application.
//!
//! This module contains the main function and handles command line arguments,
//! media processing, and terminal display.
//! The main function launches two threads, one for processing the media and
//! one for displaying the terminal.
//! The media processing thread is responsible for reading the media file,
//! processing it, and sending the processed frames to the terminal thread.
//! The terminal thread is responsible for displaying the terminal and
//! receiving the processed frames from the media processing thread.
//! The media processing thread and the terminal thread communicate via a
//! shared buffer.
mod audio;
mod common;
mod downloader;
mod msg;
mod pipeline;
mod terminal;

use audio::runner::Control as AudioControl;
use clap::Parser;
use common::errors::*;
use crossbeam_channel::{bounded, unbounded};
use either::Either;
use msg::broker::Control as MediaControl;
use pipeline::frames::FrameIterator;
use pipeline::{
    char_maps::CHARS1, frames::open_media, image_pipeline::ImagePipeline,
    runner::Control as PipelineControl,
};
use std::thread;
use terminal::Terminal;

pub type StringInfo = (String, Vec<u8>);

/// Command line arguments structure.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the file/stream to process
    #[arg(required = true, index = 1)]
    input: String,
    /// Maximum fps
    #[arg(short, long, default_value = "60.0")]
    fps: String,
    /// Custom lookup char table
    #[arg(short, long, default_value = CHARS1)]
    char_map: String,
    /// Grayscale mode
    #[arg(short, long, default_value = "false")]
    gray: bool,
    /// Experimental width modifier (emojis have 2x width)
    #[arg(short, long, default_value = "1")]
    w_mod: u32,
}

const DEFAULT_TERMINAL_SIZE: (u32, u32) = (80, 24);

use std::sync::{Arc, Barrier};
use std::thread::JoinHandle;

struct MediaProcessor {
    handles: Vec<JoinHandle<Result<(), MyError>>>,
    barrier: Arc<Barrier>,
}

impl MediaProcessor {
    pub fn new(n_threads: usize) -> Self {
        MediaProcessor {
            handles: Vec::with_capacity(n_threads),
            barrier: Arc::new(Barrier::new(n_threads)),
        }
    }

    pub fn launch_broker_thread(
        &mut self,
        rx_controls: crossbeam_channel::Receiver<MediaControl>,
        tx_controls_pipeline: Option<crossbeam_channel::Sender<PipelineControl>>,
        tx_controls_audio: Option<crossbeam_channel::Sender<AudioControl>>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let mut broker = msg::broker::MessageBroker::new(
                rx_controls,
                tx_controls_pipeline,
                tx_controls_audio,
            );
            barrier.wait();
            broker.run()
        });
        self.handles.push(handle);
        Ok(())
    }

    pub fn launch_terminal_thread(
        &mut self,
        title: String,
        gray: bool,
        rx_frames: crossbeam_channel::Receiver<Option<StringInfo>>,
        tx_controls: crossbeam_channel::Sender<MediaControl>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let mut term = Terminal::new(title, gray, rx_frames, tx_controls, barrier);
            term.run()
        });
        self.handles.push(handle);
        Ok(())
    }

    pub fn launch_pipeline_thread(
        &mut self,
        args: &Args,
        media: FrameIterator,
        fps: Option<f64>,
        tx_frames: crossbeam_channel::Sender<Option<StringInfo>>,
        rx_controls_pipeline: crossbeam_channel::Receiver<PipelineControl>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let args_fps = args
            .fps
            .parse::<f64>()
            .map_err(|err| MyError::Application(format!("{ERROR_DATA}:{err:?}")))?;
        let cmaps = args.char_map.chars().collect();
        let wmod = args.w_mod; //.clone();
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let mut runner = pipeline::runner::Runner::init(
                ImagePipeline::new(DEFAULT_TERMINAL_SIZE, cmaps),
                media,
                fps.unwrap_or(args_fps),
                tx_frames,
                rx_controls_pipeline,
                wmod,
                barrier,
            );
            runner.run()
        });
        self.handles.push(handle);
        Ok(())
    }

    pub fn launch_audio_thread(
        &mut self,
        file_path: String,
        rx_controls_audio: crossbeam_channel::Receiver<AudioControl>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let player = audio::player::AudioPlayer::new(&file_path)?;
            let mut runner = audio::runner::Runner::new(player, rx_controls_audio, barrier);
            runner.run()
        });
        self.handles.push(handle);
        Ok(())
    }

    pub fn join_threads(self) {
        for handle in self.handles {
            let _ = handle.join();
        }
    }
}

fn main() -> Result<(), MyError> {
    let args = Args::parse();

    let title = args.input.clone();

    let (media, fps, audio) = open_media(title)?;

    let num_threads = if audio.is_some() { 4 } else { 3 };

    let (tx_frames, rx_frames) = bounded::<Option<StringInfo>>(1);

    let (tx_controls, rx_controls) = unbounded::<MediaControl>();
    let (tx_controls_pipeline, rx_controls_pipeline) = unbounded::<PipelineControl>();
    let (tx_controls_audio, rx_controls_audio) = unbounded::<AudioControl>();

    let tx_controls_pipeline = Some(tx_controls_pipeline);
    let tx_controls_audio = if audio.is_some() {
        Some(tx_controls_audio)
    } else {
        None
    };

    let mut media_processor = MediaProcessor::new(num_threads);
    let _ = media_processor.launch_broker_thread(
        rx_controls,
        tx_controls_pipeline,
        tx_controls_audio,
    )?;

    let _ = media_processor.launch_terminal_thread(
        args.input.clone(),
        args.gray,
        rx_frames,
        tx_controls,
    )?;

    let _ = media_processor.launch_pipeline_thread(
        &args,
        media,
        fps,
        tx_frames,
        rx_controls_pipeline,
    )?;

    if let Some(audio) = audio {
        let title = args.input.clone();
        let file_path = if let Either::Left(audio_track) = audio.as_ref() {
            let x = audio_track.to_str().unwrap_or(&title);
            String::from(x)
        } else {
            title
        };
        let _ = media_processor.launch_audio_thread(file_path, rx_controls_audio)?;
    }

    media_processor.join_threads();

    Ok(())
}
