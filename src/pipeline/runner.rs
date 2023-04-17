//! The `runner` module contains the Runner struct and related functionality to
//! control and run ASCII animations.
//!
//! The `Runner` struct is responsible for handling the image pipeline,
//! processing frames, managing playback state, and controlling the frame rate.
//! It also handles commands for pausing/continuing, resizing, and changing
//! character maps during playback.
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
    /// The Runner does not process new frames, but can update the terminal by
    /// processing the last frame again if charset or dimension change.
    Paused,
    /// The Runner was stopped by a command and will cease processing frames,
    /// and eventually exit.
    Stopped,
}

/// The `Runner` struct handles the image pipeline, processing frames, managing
/// playback state, and controlling the frame rate. It also handles commands for
/// pausing/continuing, resizing, and changing character maps during playback.
pub struct Runner {
    /// The image pipeline responsible for processing images.
    pipeline: ImagePipeline,
    /// The FrameIterator that handles iterating through frames.
    media: FrameIterator,
    /// The target frames per second (frame rate) for the Runner.
    fps: u64,
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
    /// (UNUSED) Command to set grayscale mode. We always extract
    /// rgb+grayscale from image, the terminal is responsible for the correct
    /// render mode.
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
    /// * `string_buffer` - A shared buffer for sending processed frames as strings.
    /// * `condvar` - A condition variable to notify that a new frame is ready.
    /// * `commands_buffer` - A shared buffer for sending control commands to the Runner.
    /// * `w_mod` - The width modifier (use 2 for emojis).
    /// * `char_maps` - A collection of character maps available for the image pipeline.
    /// * `last_frame` - The last frame that was processed by the Runner.
    pub fn init(
        pipeline: ImagePipeline,
        media: FrameIterator,
        fps: u64,
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

    /// Determines if it's time to send the next frame based on the current time and the target
    /// frame rate. Updates the time_count accordingly and sleeps for a short duration if it's not
    /// time to send the next frame.
    ///
    /// # Arguments
    ///
    /// * `time_count` - A mutable reference to the time counter used for frame rate control.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether it's time to send the next frame.
    fn time_to_send_next_frame(&self, time_count: &mut std::time::Instant) -> bool {
        if std::time::Instant::now()
            .duration_since(*time_count)
            .as_micros()
            < 1_000_000_u64.checked_div(self.fps).unwrap_or(0).into()
        {
            return false;
        }
        *time_count += Duration::from_micros(1_000_000_u64.checked_div(self.fps).unwrap_or(0));
        true
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
    /// A String containing the ASCII representation of the processed image.
    fn process_frame(&mut self, frame: &DynamicImage) -> Result<StringInfo, MyError> {
        let procimage = self.pipeline.resize(frame)?;
        let grayimage = procimage.clone().into_luma8();
        let rgb_info = procimage.into_rgb8().to_vec();
        Ok((self.pipeline.to_ascii(&grayimage), rgb_info))
    }

    /// The main function responsible for running the animation. It processes control commands,
    /// updates the state of the Runner, processes frames, and sends the resulting ASCII strings
    /// to the string buffer.
    pub fn run(&mut self) -> Result<(), MyError> {
        let mut time_count = std::time::Instant::now();

        while self.state != State::Stopped {
            let frame_needs_refresh = self.process_control_commands();

            if self.should_process_frame(&mut time_count) {
                let frame = self.get_current_frame();

                // Use crossbeam-channel's select! macro to check if terminal is ready for the next frame
                select! {
                    send(self.tx_frames, None) -> _ => {
                        let string_info = self.process_current_frame(frame.as_ref(), frame_needs_refresh);
                        let _ = self.tx_frames.try_send(string_info); // Best effort send. If the buffer is full the frame will be dropped
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

    /// Processes control commands from the commands buffer and updates the Runner state and
    /// other properties accordingly. Returns a boolean indicating if the frame needs to be
    /// refreshed.
    ///
    /// # Returns
    ///
    /// A boolean indicating if the frame needs to be refreshed.
    fn process_control_commands(&mut self) -> bool {
        // Get the next control event
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
    /// A boolean indicating whether a frame should be processed.
    fn should_process_frame(&self, time_count: &mut std::time::Instant) -> bool {
        self.time_to_send_next_frame(time_count)
            && (self.state == State::Running || self.state == State::Paused)
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
    /// An Optional StringInfo tuple containing the ASCII representation of the processed frame and RGB info.
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
