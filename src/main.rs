mod cli;
mod pipeline;
mod terminal;
use crate::pipeline::char_maps::SHORT;
use crate::pipeline::image_pipeline::ImagePipeline;
use clap::Parser;
use crossterm::terminal as ct_terminal;
use crossterm::Result;
use image::DynamicImage;
use pipeline::frames::{open_media, FrameIterator};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use terminal::Terminal;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the file to process
    #[arg(short, long)]
    #[clap(default_value = "assets/homer.jpg")]
    in_file: String,
    /// Maximum fps
    #[arg(short, long)]
    fps: Option<u64>,
    /// Custom lookup char table
    #[arg(short, long)]
    #[clap(default_value = SHORT)]
    char_lookup: String,
    /// Width modifier (emojis have 2x width, default to 1)
    #[arg(long)]
    #[clap(default_value = "1")]
    w_mod: u16,
    /// Height modifier (just in case, default to 1)
    #[arg(long)]
    #[clap(default_value = "1")]
    h_mod: u16,
}

pub enum Control {
    Refresh, // Request a refresh
    PauseContinue,
    Exit,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let buffer = Arc::new(Mutex::new(VecDeque::new()));
    let buffer_condvar = Arc::new(Condvar::new());
    let buffer_condvar_clone = Arc::clone(&buffer_condvar);

    let mut media: FrameIterator = open_media(args.in_file.clone()).unwrap();

    let buffer_controls = Arc::new(Mutex::new(VecDeque::<Control>::new()));

    let buffer_controls_pipeline = Arc::clone(&buffer_controls);
    let buffer_controls_terminal = Arc::clone(&buffer_controls);

    let buffer_thread_pipeline = Arc::clone(&buffer);
    let buffer_thread_terminal = Arc::clone(&buffer);

    // Launch Pipeline thread
    // TODO: Move this to a separate file
    let handle_thread_pipeline = thread::spawn(move || loop {
        let (width, height) = ct_terminal::size().unwrap();

        let mut image_pipeline = ImagePipeline::new(
            (width.into(), height.into()),
            args.char_lookup.chars().collect(),
            // (1, 1),
        );
        // Initialize time variable
        let mut time_count = std::time::Instant::now();
        // DT depends on fps
        let mut dt: u64 = 0;
        if let Some(fps) = args.fps {
            dt = 1_000_000_u64.checked_div(fps).unwrap_or(0);
        }
        let mut paused = false;
        let mut refresh = true;
        let mut last_frame: Option<DynamicImage> = None;
        'outer: loop {
            if std::time::Instant::now()
                .duration_since(time_count)
                .as_micros()
                < dt.into()
            {
                thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            time_count += Duration::from_micros(dt);
            if !paused {
                if let Some(frame) = media.next() {
                    last_frame = Some(frame.clone());
                    let (width, height) = ct_terminal::size().unwrap();
                    image_pipeline.set_target_resolution(
                        (width / args.w_mod).into(),
                        (height / args.h_mod).into(),
                    );
                    let procimage = image_pipeline.process(&frame);
                    let output = image_pipeline.to_ascii(&procimage);
                    let mut buffer_guard = buffer_thread_pipeline.lock().unwrap();
                    buffer_guard.push_back(output.clone());
                    buffer_condvar_clone.notify_one();
                }
            }
            if refresh && last_frame.is_some() {
                let (width, height) = ct_terminal::size().unwrap();
                image_pipeline.set_target_resolution(
                    (width / args.w_mod).into(),
                    (height / args.h_mod).into(),
                );
                let procimage = image_pipeline.process(&last_frame.clone().unwrap());
                let output = image_pipeline.to_ascii(&procimage);
                let mut buffer_guard = buffer_thread_pipeline.lock().unwrap();
                buffer_guard.push_back(output.clone());
                buffer_condvar_clone.notify_one();
                refresh = false;
            }

            // Check for control commands
            let mut buffer_controls_guard = buffer_controls_pipeline.lock().unwrap();
            if let Some(control) = buffer_controls_guard.pop_front() {
                match control {
                    // Wait for continue command
                    Control::PauseContinue => {
                        paused = !paused;
                    }
                    Control::Exit => return,
                    Control::Refresh => refresh = true,
                }
            }
        }
    });

    // Launch Terminal Thread
    let handle_thread_terminal = thread::spawn(move || {
        let t2 = Terminal::new(args.in_file);
        t2.run(
            buffer_thread_terminal,
            buffer_condvar,
            buffer_controls_terminal,
        )
        .unwrap();
    });
    // Wait for threads to finish
    handle_thread_terminal.join().unwrap();
    handle_thread_pipeline.join().unwrap();

    Ok(())
}
