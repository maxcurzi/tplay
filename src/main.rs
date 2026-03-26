//! Main module for the application.
//!
//! This module contains the main function and handles command line arguments,
//! and launches the audio and image pipelines as well as the terminal.
mod audio;
mod common;
mod downloader;
mod msg;
mod pipeline;
mod subtitle;
mod terminal;

use audio::runner::Control as AudioControl;
use clap::Parser;
use common::{errors::*, sync::PlaybackClock};
use crossbeam_channel::{bounded, unbounded};
use either::Either;
use msg::broker::Control as MediaControl;
use pipeline::{
    char_maps::CHARS3, frames::open_media, frames::FrameIterator, image_pipeline::ImagePipeline,
    runner::Control as PipelineControl, runner::RunnerOptions,
};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use subtitle::{extract_subtitles, SubtitleManager};
use terminal::Terminal;

pub type StringInfo = (String, Vec<u8>);

const AFTER_HELP: &str = "\
\x1b[1;4mPlayback controls:\x1b[0m
  0-9           Change character map
  space         Toggle pause/unpause
  g             Toggle grayscale/color
  m             Toggle mute/unmute
  ← / →         Seek backward/forward 5 seconds
  j / l           Seek backward/forward 10 seconds
  q             Quit

\x1b[1;4mSubtitle controls:\x1b[0m
  c             Cycle through subtitle tracks
  C (Shift+c)   Toggle subtitles on/off

\x1b[1;4mPlayback Speed Control:\x1b[0m
  [ / ]         Decrease/increase speed by 0.25x
  , / .         Decrease/increase speed by 0.1x
  \\             Reset speed to 1.0x";

/// Command line arguments structure.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, after_help = AFTER_HELP)]
struct Args {
    /// Name of the file/stream to process
    #[arg(required = true, index = 1)]
    input: String,
    /// Force a user-specified FPS
    #[arg(short, long)]
    fps: Option<String>,
    /// import YT cookies from browser (default: firefox)
    #[arg(short, long, required = false)]
    browser: Option<String>,
    /// Loop playing of video/gif
    #[arg(short, long, default_value = "false")]
    loop_playback: bool,
    /// Custom lookup char table
    #[arg(short, long, default_value = CHARS3)]
    char_map: String,
    /// Grayscale mode
    #[arg(short, long, default_value = "false")]
    gray: bool,
    /// Experimental width modifier (emojis have 2x width)
    #[arg(short, long, default_value = "1")]
    w_mod: u32,
    /// Experimental frame skip flag
    #[arg(short, long, default_value = "false")]
    allow_frame_skip: bool,
    /// Experimental flag to add newlines
    #[arg(short, long, default_value = "false")]
    new_lines: bool,
    /// Exit automatically when the media ends
    #[arg(long, short = 'x', default_value = "false")]
    auto_exit: bool,
    /// Stretch video to fill terminal (ignore aspect ratio)
    #[arg(short, long, default_value = "false")]
    stretch: bool,
}

const DEFAULT_TERMINAL_SIZE: (u32, u32) = (80, 24);
const DEFAULT_FPS: f64 = 30.0;
const DEFAULT_BROWSER: &str = "firefox";

use std::sync::Barrier;
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
            broker.run(barrier)
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
        subtitle_text: Option<Arc<RwLock<String>>>,
        local_subtitles: Option<SubtitleManager>,
        playback_clock: Option<Arc<PlaybackClock>>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let mut term = Terminal::new(
                title,
                gray,
                rx_frames,
                tx_controls,
                subtitle_text,
                local_subtitles,
                playback_clock,
            );
            term.run(barrier)
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
        tx_controls: crossbeam_channel::Sender<MediaControl>,
        playback_clock: Option<Arc<PlaybackClock>>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let mut use_fps = DEFAULT_FPS;
        if let Some(fps) = fps {
            use_fps = fps;
        }
        if let Some(fps) = &args.fps {
            use_fps = fps
                .parse::<f64>()
                .map_err(|err| MyError::Application(format!("{ERROR_DATA}:{err:?}")))?;
        }
        let cmaps = args.char_map.chars().collect();
        let w_mod = args.w_mod;
        let loop_playback = args.loop_playback;
        let auto_exit = args.auto_exit;
        let allow_frame_skip = args.allow_frame_skip;
        let new_lines = args.new_lines;
        let preserve_aspect_ratio = !args.stretch;
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let mut runner = pipeline::runner::Runner::new(
                ImagePipeline::new(DEFAULT_TERMINAL_SIZE, cmaps, new_lines),
                media,
                tx_frames,
                rx_controls_pipeline,
                tx_controls,
                RunnerOptions {
                    fps: use_fps,
                    w_mod,
                    loop_playback,
                    auto_exit,
                    preserve_aspect_ratio,
                },
                playback_clock,
            );
            runner.run(barrier, allow_frame_skip)
        });
        self.handles.push(handle);
        Ok(())
    }

    pub fn launch_audio_thread(
        &mut self,
        file_path: String,
        rx_controls_audio: crossbeam_channel::Receiver<AudioControl>,
        subtitle_text: Option<Arc<RwLock<String>>>,
        playback_clock: Option<Arc<PlaybackClock>>,
    ) -> Result<(), MyError> {
        let barrier = Arc::clone(&self.barrier);
        let handle = thread::spawn(move || -> Result<(), MyError> {
            let player = audio::player::AudioPlayer::new(&file_path)?;
            let mut runner = audio::runner::Runner::new(player, rx_controls_audio, subtitle_text, playback_clock);
            runner.run(barrier)
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
    let browser = if args.browser.clone().is_some() {
        args.browser.clone().unwrap()
    } else {
        String::from(DEFAULT_BROWSER)
    };

    let media_data = open_media(title.clone(), browser)?;
    let media = media_data.frame_iter;
    let fps = media_data.fps;
    let audio = media_data.audio_path;

    let is_local_file = Path::new(&args.input).exists();
    
    let local_subtitles = if is_local_file {
        let tracks = extract_subtitles(Path::new(&args.input));
        if tracks.is_empty() {
            None
        } else {
            Some(SubtitleManager::new(tracks))
        }
    } else {
        None
    };

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

    let subtitle_text: Option<Arc<RwLock<String>>> = if audio.is_some() {
        Some(Arc::new(RwLock::new(String::new())))
    } else {
        None
    };

    let playback_clock: Option<Arc<PlaybackClock>> = if audio.is_some() {
        Some(Arc::new(PlaybackClock::new()))
    } else {
        None
    };

    let mut media_processor = MediaProcessor::new(num_threads);
    media_processor.launch_broker_thread(rx_controls, tx_controls_pipeline, tx_controls_audio)?;

    media_processor.launch_terminal_thread(
        args.input.clone(),
        args.gray,
        rx_frames,
        tx_controls.clone(),
        subtitle_text.clone(),
        local_subtitles,
        playback_clock.clone(),
    )?;

    media_processor.launch_pipeline_thread(
        &args,
        media,
        fps,
        tx_frames,
        rx_controls_pipeline,
        tx_controls,
        playback_clock.clone(),
    )?;

    if let Some(audio) = &audio {
        let title = args.input.clone();
        let file_path = match audio.as_ref() {
            Either::Left(audio_track) => {
                String::from(audio_track.to_str().unwrap_or(&title))
            }
            Either::Right(path_string) => path_string.clone(),
        };
        media_processor.launch_audio_thread(file_path, rx_controls_audio, subtitle_text, playback_clock)?;
    }

    media_processor.join_threads();

    Ok(())
}
