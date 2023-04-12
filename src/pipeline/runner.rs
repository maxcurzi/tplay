//! The `runner` module contains the Runner struct and related functionality to
//! control and run ASCII animations.
//!
//! The `Runner` struct is responsible for handling the image pipeline,
//! processing frames, managing playback state, and controlling the frame rate.
//! It also handles commands for pausing/continuing, resizing, and changing
//! character maps during playback.
use image::DynamicImage;

use crate::pipeline::char_maps::{LONG1, LONG2, SHORT1, SHORT2};

use super::frames::FrameIterator;
use super::image_pipeline::ImagePipeline;
use std::collections::VecDeque;
use std::sync::Condvar;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Running,
    Paused,
    Stopped,
}

pub struct Runner {
    pipeline: ImagePipeline,
    media: FrameIterator,
    fps: u64,
    state: State,
    string_buffer: Arc<Mutex<VecDeque<String>>>,
    condvar: Arc<Condvar>,
    commands_buffer: Arc<Mutex<VecDeque<Control>>>,
    w_mod: u32, // Width modifier (use 2 for emojis)
    char_maps: Vec<Vec<char>>,
}

#[derive(Debug, PartialEq)]
pub enum Control {
    PauseContinue,
    Exit,
    SetCharMap(u32),
    Resize(u16, u16),
}
const MAX_CAPACITY: usize = 1;

impl Runner {
    pub fn init(
        pipeline: ImagePipeline,
        media: FrameIterator,
        fps: u64,
        string_buffer: Arc<Mutex<VecDeque<String>>>,
        condvar: Arc<Condvar>,
        commands_buffer: Arc<Mutex<VecDeque<Control>>>,
        w_mod: u32,
    ) -> Self {
        let char_maps: Vec<Vec<char>> = vec![
            pipeline.char_lookup.clone(),
            SHORT1.to_string().chars().collect(),
            SHORT2.to_string().chars().collect(),
            LONG1.to_string().chars().collect(),
            LONG2.to_string().chars().collect(),
        ];

        Self {
            pipeline,
            media,
            fps,
            state: State::Running,
            string_buffer,
            condvar,
            commands_buffer,
            w_mod,
            char_maps,
        }
    }

    fn time_to_send_next_frame(&self, time_count: &mut std::time::Instant) -> bool {
        if std::time::Instant::now()
            .duration_since(*time_count)
            .as_micros()
            < 1_000_000_u64.checked_div(self.fps).unwrap_or(0).into()
        {
            thread::sleep(Duration::from_millis(1));
            return false;
        }
        *time_count += Duration::from_micros(1_000_000_u64.checked_div(self.fps).unwrap_or(0));
        true
    }

    fn process_frame(&mut self, frame: &DynamicImage) -> String {
        let procimage = self.pipeline.process(frame);
        self.pipeline.to_ascii(&procimage)
    }

    pub fn run(&mut self) {
        // Initialize local useful variables
        let mut time_count = std::time::Instant::now();
        let mut last_frame: Option<DynamicImage> = None;
        let mut refresh = false;

        while self.state != State::Stopped {
            // Check for control commands
            let mut buffer_controls_guard = self.commands_buffer.lock().unwrap();
            if let Some(control) = buffer_controls_guard.pop_front() {
                match control {
                    // Wait for continue command
                    Control::PauseContinue => match self.state {
                        State::Running => {
                            self.state = State::Paused;
                        }
                        State::Paused => {
                            self.state = State::Running;
                        }
                        _ => {}
                    },
                    Control::Exit => self.state = State::Stopped,
                    Control::Resize(width, height) => {
                        let _ = self.pipeline.set_target_resolution(
                            (width / self.w_mod as u16).into(),
                            height.into(),
                        );
                        refresh = true;
                    }
                    Control::SetCharMap(char_map) => {
                        self.pipeline.char_lookup = self.char_maps
                            [(char_map % self.char_maps.len() as u32) as usize]
                            .clone();
                        refresh = true;
                    }
                }
            }
            drop(buffer_controls_guard);
            if self.time_to_send_next_frame(&mut time_count)
                && (self.state == State::Running || self.state == State::Paused)
            {
                // Convert new frame
                let frame = match self.state {
                    State::Running => self.media.next(),
                    State::Paused | State::Stopped => None,
                };
                let string_out = match frame {
                    Some(frame) => {
                        last_frame = Some(frame.clone());
                        self.process_frame(&frame)
                    }
                    None => {
                        if last_frame.is_some() && refresh {
                            refresh = false;
                            self.process_frame(&last_frame.clone().unwrap())
                        } else {
                            String::new()
                        }
                    }
                };
                let mut buffer_guard = self.string_buffer.lock().unwrap();
                buffer_guard.push_back(string_out);
                self.condvar.notify_one();
            }
        }
    }
}
