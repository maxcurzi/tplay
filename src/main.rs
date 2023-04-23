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
use pipeline::{
    char_maps::CHARS1,
    frames::{open_media, FrameIterator},
    image_pipeline::ImagePipeline,
    runner::Control as PipelineControl,
};
use std::path::Path;
use std::thread;
use std::time::Duration;
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

/// Main function for the application.
///
/// This function parses command line arguments, opens the media, initializes the
/// pipeline and terminal threads, and then waits for them to finish.
fn main() -> Result<(), MyError> {
    let args = Args::parse();

    let args_fps = args
        .fps
        .parse::<f64>()
        .map_err(|err| MyError::Application(format!("{ERROR_DATA}:{err:?}")))?;
    let title = args.input.clone();

    let (media, fps, audio) = open_media(title)?;

    let num_threads = if audio.is_some() { 4 } else { 3 };
    let mut handles = Vec::with_capacity(num_threads);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(num_threads));

    // Set up a channel for passing frames and controls
    let (tx_frames, rx_frames) = bounded::<Option<StringInfo>>(1);

    // From to broker
    let (tx_controls, rx_controls) = unbounded::<MediaControl>();
    let (tx_controls_pipeline, rx_controls_pipeline) = unbounded::<PipelineControl>();
    let (tx_controls_audio, rx_controls_audio) = unbounded::<AudioControl>();

    let tx_controls_pipeline = Some(tx_controls_pipeline);
    let tx_controls_audio = if audio.is_some() {
        Some(tx_controls_audio)
    } else {
        None
    };

    // Launch message broker
    let broker_barrier = std::sync::Arc::clone(&barrier);
    let handle_thread_broker = thread::spawn(move || -> Result<(), MyError> {
        let mut broker =
            msg::broker::MessageBroker::new(rx_controls, tx_controls_pipeline, tx_controls_audio);
        broker_barrier.wait();
        broker.run()
    });
    handles.push(handle_thread_broker);

    // Launch Terminal Thread
    let title = args.input.clone();
    let term_barrier = std::sync::Arc::clone(&barrier);
    let handle_thread_terminal = thread::spawn(move || -> Result<(), MyError> {
        let mut term = Terminal::new(title, args.gray, rx_frames, tx_controls, term_barrier);
        term.run()
    });
    handles.push(handle_thread_terminal);

    // Launch Image Pipeline Thread
    let pipeline_barrier = std::sync::Arc::clone(&barrier);
    let handle_thread_pipeline = thread::spawn(move || -> Result<(), MyError> {
        let mut runner = pipeline::runner::Runner::init(
            ImagePipeline::new(DEFAULT_TERMINAL_SIZE, args.char_map.chars().collect()),
            media,
            fps.unwrap_or(args_fps),
            tx_frames,
            rx_controls_pipeline,
            args.w_mod,
            pipeline_barrier,
        );
        runner.run()
    });
    handles.push(handle_thread_pipeline);

    // Launch Audio Thread
    let boxed_temp_file = Box::new(audio);
    if boxed_temp_file.is_some() {
        let audio_track = boxed_temp_file.as_ref().as_ref();
        let title = args.input.clone();
        let file_path = if let Some(Either::Left(audio_track)) = audio_track {
            let x = audio_track.to_str().unwrap_or(&title);
            String::from(x)
        } else {
            title
        };
        let audio_barrier = std::sync::Arc::clone(&barrier);
        let handle_thread_audio = thread::spawn(move || -> Result<(), MyError> {
            let player = audio::player::AudioPlayer::new(&file_path)?;
            let mut runner = audio::runner::Runner::new(player, rx_controls_audio, audio_barrier);
            runner.run()
        });
        handles.push(handle_thread_audio);
    }

    // Wait for threads to finish
    for handle in handles {
        let _ = handle.join(); //.expect("a thread panicked");
    }
    // dbg!(boxed_temp_file);
    Ok(())
}
