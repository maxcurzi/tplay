//! The `runner` module contains the Runner struct and related functionality to control and run
//! ASCII animations.
//!
//! The `Runner` struct is responsible for handling the image pipeline, processing frames, managing
//! playback state, and controlling the frame rate. It also handles commands for pausing/continuing,
//! resizing, and changing character maps during playback.
use super::{frames::FrameIterator, image_pipeline::ImagePipeline};
use crate::{common::errors::MyError, pipeline::char_maps::*, StringInfo};
use crossbeam_channel::{select, Receiver, Sender};
use image::DynamicImage;
use std::{thread, time::Duration};

/// Represents the playback state of the Runner.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    /// The Runner is currently reading and processing new  frames.
    Running,
    /// The Runner does not process new frames, but can update the terminal by processing the last
    /// frame again if charset or dimension change.
    Paused,
    /// The Runner was stopped by a command and will cease processing frames, and eventually exit.
    Stopped,
}

/// The `Runner` struct handles the image pipeline, processing frames, managing playback state, and
/// controlling the frame rate. It also handles commands for pausing/continuing, resizing, and
/// changing character maps during playback.
pub struct Runner {
    /// The image pipeline responsible for processing images.
    pipeline: ImagePipeline,
    /// The FrameIterator that handles iterating through frames.
    media: FrameIterator,
    /// The target frames per second (frame rate) for the Runner.
    fps: f64,
    /// The current playback state of the Runner.
    state: State,
    /// A channel for receiving processed frames as strings.
    tx_frames: Sender<Option<StringInfo>>,
    /// A channel for sending control commands to the Runner.
    rx_controls: Receiver<Control>,
    /// The width modifier (use 2 for emojis).
    w_mod: u32,
    /// A collection of character maps available for the image pipeline.
    char_maps: Vec<Vec<char>>,
    /// The last frame that was processed by the Runner.
    last_frame: Option<DynamicImage>,
}

/// Enum representing the different control commands that can be sent to the Runner.
#[derive(Debug, PartialEq)]
pub enum Control {
    /// Command to toggle between pause and continue playback.
    PauseContinue,
    /// Command to stop the playback and exit the Runner.
    Exit,
    /// Command to set the character map used by the image pipeline.
    /// The argument represents the index of the desired character map.
    SetCharMap(u32),
    /// Command to resize the target resolution of the image pipeline.
    /// The arguments represent the new target width and height, respectively.
    Resize(u16, u16),
    /// Command to set grayscale mode. We always extract rgb+grayscale from image, the
    /// terminal is responsible for the correct render mode.
    SetGrayscale(bool),
}

impl Runner {
    /// Initializes a new Runner instance.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - The image pipeline responsible for processing images.
    /// * `media` - The FrameIterator that handles iterating through frames.
    /// * `fps` - The target frames per second (frame rate) for the Runner.
    /// * `tx_frames` - A channel for receiving processed frames as strings.
    /// * `rx_controls` - A channel for sending control commands to the Runner.
    /// * `w_mod` - The width modifier (use 2 for emojis).
    pub fn init(
        pipeline: ImagePipeline,
        media: FrameIterator,
        fps: f64,
        tx_frames: Sender<Option<StringInfo>>,
        rx_controls: Receiver<Control>,
        w_mod: u32,
    ) -> Self {
        let char_maps: Vec<Vec<char>> = vec![
            pipeline.char_map.clone(),
            CHARS1.to_string().chars().collect(),
            CHARS2.to_string().chars().collect(),
            CHARS3.to_string().chars().collect(),
            SOLID.to_string().chars().collect(),
            DOTTED.to_string().chars().collect(),
            GRADIENT.to_string().chars().collect(),
            BLACKWHITE.to_string().chars().collect(),
            BW_DOTTED.to_string().chars().collect(),
            BRAILLE.to_string().chars().collect(),
        ];

        Self {
            pipeline,
            media,
            fps,
            state: State::Running,
            tx_frames,
            rx_controls,
            w_mod,
            char_maps,
            last_frame: None,
        }
    }

    /// The main function responsible for running the animation.
    ///
    /// It processes control commands, updates the state of the Runner, processes frames, and sends
    /// the resulting ASCII strings to the string buffer.
    ///
    /// # Returns
    ///
    /// An empty Result.
    pub fn run(
        &mut self,
        barrier: std::sync::Arc<std::sync::Barrier>,
        allow_frame_skip: bool,
    ) -> Result<(), MyError> {
        barrier.wait();
        let mut time_count = std::time::Instant::now();
        while self.state != State::Stopped {
            let frame_needs_refresh = self.process_control_commands();

            let (should_process_frame, frames_to_skip) = self.should_process_frame(&mut time_count);
            if should_process_frame {
                if frames_to_skip > 0 && allow_frame_skip {
                    self.media.skip_frames(frames_to_skip);
                }
                let frame = self.get_current_frame();

                // Check if terminal is ready for the next frame
                select! {
                    send(self.tx_frames, None) -> _ => {
                        let string_info = self.process_current_frame(frame.as_ref(), frame_needs_refresh);
                        // Best effort send. If the buffer is full the frame will be dropped
                        let _ = self.tx_frames.try_send(string_info);
                    },
                    default(Duration::from_millis(5)) => {
                        // Terminal may be struggling to keep up. Give it some slack!
                    }
                }
            } else {
                // Be a nice thread
                thread::yield_now();
            }
        }
        Ok(())
    }

    /// Processes the given frame using the image pipeline and converts the processed image to an
    /// ASCII string representation.
    ///
    /// # Arguments
    ///
    /// * `frame` - A reference to the DynamicImage to be processed.
    ///
    /// # Returns
    ///
    /// A Result containing a tuple of the ASCII string representation of the processed image and
    /// the RGB data of the processed image.
    fn process_frame(&mut self, frame: &DynamicImage) -> Result<StringInfo, MyError> {
        let procimage = self.pipeline.resize(frame)?;
        let grayimage = procimage.clone().into_luma8();
        let rgb_info = procimage.into_rgb8().to_vec();

        // Add newlines to the rgb_info to match the ascii string These are not
        // really needed, but it's important if you want to copy/paste the
        // output and preserve the aspect.
        if self.pipeline.new_lines {
            let mut rgb_info_newline =
                Vec::with_capacity(rgb_info.len() + 6 * self.pipeline.target_resolution.0 as usize);

            for (i, pixel) in rgb_info.chunks(3).enumerate() {
                rgb_info_newline.extend_from_slice(pixel);
                if (i + 1) % self.pipeline.target_resolution.0 as usize == 0 {
                    rgb_info_newline.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                }
            }
            return Ok((self.pipeline.to_ascii(&grayimage), rgb_info_newline));
        }
        Ok((self.pipeline.to_ascii(&grayimage), rgb_info))
    }

    /// Processes control commands from the commands buffer and updates the Runner state and
    /// other properties accordingly.
    ///
    /// # Returns
    ///
    /// A boolean indicating if the frame needs to be refreshed.
    fn process_control_commands(&mut self) -> bool {
        let mut needs_refresh = false;

        // If we have control events, process them
        while let Ok(control) = self.rx_controls.recv_timeout(Duration::from_millis(1)) {
            needs_refresh = true;
            match control {
                Control::PauseContinue => self.toggle_pause(),
                Control::Exit => self.state = State::Stopped,
                Control::Resize(width, height) => {
                    self.resize_pipeline(width, height);
                }
                Control::SetCharMap(char_map) => {
                    self.set_char_map(char_map);
                }
                Control::SetGrayscale(_) => { /* ignore */ }
            }
        }
        needs_refresh
    }

    /// Toggles the playback state of the Runner between `Running` and `Paused`.
    fn toggle_pause(&mut self) {
        match self.state {
            State::Running => self.state = State::Paused,
            State::Paused => self.state = State::Running,
            _ => {}
        }
    }

    /// Resizes the image pipeline's target resolution based on the provided width and height.
    ///
    /// # Arguments
    ///
    /// * `width` - The new target width.
    /// * `height` - The new target height.
    fn resize_pipeline(&mut self, width: u16, height: u16) {
        let _ = self
            .pipeline
            .set_target_resolution((width / self.w_mod as u16).into(), height.into());
    }

    /// Sets the character map for the image pipeline based on the provided index.
    ///
    /// # Arguments
    ///
    /// * `char_map` - The index of the character map to use.
    fn set_char_map(&mut self, char_map: u32) {
        self.pipeline.char_map =
            self.char_maps[(char_map % self.char_maps.len() as u32) as usize].clone();
    }

    /// Determines if a frame should be processed based on the current time and the Runner's state.
    ///
    /// # Arguments
    ///
    /// * `time_count` - A mutable reference to the time counter used for frame rate control.
    ///
    /// # Returns
    ///
    /// A tuple containing a boolean indicating whether a frame should be processed, and the number
    /// of frames to skip if we are behind schedule.
    fn should_process_frame(&self, time_count: &mut std::time::Instant) -> (bool, usize) {
        let (time_to_send_next_frame, frames_to_skip) = self.time_to_send_next_frame(time_count);

        if time_to_send_next_frame && (self.state == State::Running || self.state == State::Paused)
        {
            (true, frames_to_skip)
        } else {
            (false, 0)
        }
    }

    /// Determines if the next frame should be sent based on the current time and the Runner's
    /// frame rate.
    ///
    /// # Arguments
    ///
    /// * `time_count` - A mutable reference to the time counter used for frame rate control.
    ///
    /// # Returns
    ///
    /// A tuple containing a boolean indicating whether the next frame should be sent, and the
    /// number of frames to skip if we are behind schedule.
    fn time_to_send_next_frame(&self, time_count: &mut std::time::Instant) -> (bool, usize) {
        let target_frame_duration = Duration::from_nanos(1_000_000_000_u64 / self.fps as u64);
        let elapsed_time = time_count.elapsed();

        if elapsed_time >= target_frame_duration {
            let frames_to_skip =
                (elapsed_time.as_nanos() / target_frame_duration.as_nanos()) as usize - 1;
            *time_count += target_frame_duration * (frames_to_skip as u32 + 1);
            (true, frames_to_skip)
        } else {
            (false, 0)
        }
    }

    /// Retrieves the current frame based on the Runner's state.
    ///
    /// # Returns
    ///
    /// An Option containing a DynamicImage if the Runner's state is `Running`, or None otherwise.
    fn get_current_frame(&mut self) -> Option<DynamicImage> {
        match self.state {
            State::Running => self.media.next(),
            State::Paused | State::Stopped => self.last_frame.clone(),
        }
    }

    /// Processes the current frame, if available, and returns the resulting ASCII string. If the
    /// frame is not available or doesn't need to be processed, it returns None.
    ///
    /// # Arguments
    ///
    /// * `frame` - An Option containing a reference to the current DynamicImage, or None.
    /// * `refresh` - A boolean indicating if the frame needs to be refreshed.
    ///
    /// # Returns
    ///
    /// An Optional StringInfo tuple containing the ASCII representation of the processed frame and
    /// RGB info.
    fn process_current_frame(
        &mut self,
        frame: Option<&DynamicImage>,
        refresh: bool,
    ) -> Option<StringInfo> {
        match frame {
            Some(frame) => {
                self.last_frame = Some(frame.clone());
                if let Ok(string_info) = self.process_frame(frame) {
                    return Some(string_info);
                }
                None
            }
            None => {
                if self.last_frame.is_some() && refresh {
                    if let Ok(string_info) = self.process_frame(
                        &self
                            .last_frame
                            .clone()
                            .expect("Last frame should be available"),
                    ) {
                        return Some(string_info);
                    }
                }
                None
            }
        }
    }
}
