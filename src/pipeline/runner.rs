use image::DynamicImage;

use crate::pipeline::char_maps::{LONG, LONG_2, SHORT, SHORT_EXT};

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
    internal_string_buffer: VecDeque<String>,
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
        let internal_string_buffer = VecDeque::with_capacity(MAX_CAPACITY);
        let char_maps: Vec<Vec<char>> = vec![
            pipeline.char_lookup.clone(),
            SHORT.to_string().chars().collect(),
            SHORT_EXT.to_string().chars().collect(),
            LONG.to_string().chars().collect(),
            LONG_2.to_string().chars().collect(),
        ];

        Self {
            pipeline,
            media,
            fps,
            state: State::Running,
            string_buffer,
            condvar,
            commands_buffer,
            internal_string_buffer,
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

    fn process_frame(&mut self, frame: &DynamicImage) {
        let procimage = self.pipeline.process(frame);
        let output = self.pipeline.to_ascii(&procimage);
        self.internal_string_buffer.push_back(output);
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
            // if self.internal_string_buffer.len() < MAX_CAPACITY {
            //     // Convert new frame
            //     let frame = match self.state {
            //         State::Running => self.media.next(),
            //         State::Paused | State::Stopped => None,
            //     };
            //     if let Some(frame) = frame {
            //         last_frame = Some(frame.clone());
            //         self.process_frame(&frame);
            //     } else if last_frame.is_some() && refresh {
            //         self.process_frame(&last_frame.clone().unwrap());
            //         refresh = false;
            //     }
            // } else {
            //     // String buffer isn't being consumed fast enough, remove oldest frame
            //     self.internal_string_buffer.pop_front();
            // }

            if self.time_to_send_next_frame(&mut time_count)
                && (self.state == State::Running || self.state == State::Paused)
            {
                // Do not buffer more than one frame, and
                // discard it if it's not read. Keeps playback stable but will
                // skip frames if the terminal is too slow
                self.internal_string_buffer.drain(..);

                // Convert new frame
                let frame = match self.state {
                    State::Running => self.media.next(),
                    State::Paused | State::Stopped => None,
                };
                if let Some(frame) = frame {
                    last_frame = Some(frame.clone());
                    self.process_frame(&frame);
                } else if last_frame.is_some() && refresh {
                    self.process_frame(&last_frame.clone().unwrap());
                    refresh = false;
                }
                let mut buffer_guard = self.string_buffer.lock().unwrap();
                if self.internal_string_buffer.len() > 0 {
                    buffer_guard.push_back(self.internal_string_buffer.pop_back().unwrap());
                    self.condvar.notify_one();
                }
            }
        }
    }
}
