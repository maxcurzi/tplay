//! The `terminal` module provides functionality for displaying an animation in
//! the terminal and handling user input events such as pausing/continuing,
//! resizing, and changing character maps.
use crate::{common::errors::*, msg::broker::Control as MediaControl, StringInfo};
use crossbeam_channel::{Receiver, Sender};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use std::{
    io::{stdout, Result as IOResult, Write},
    time::Duration,
};

/// Represents the playback state of the Terminal.
#[derive(PartialEq)]
enum State {
    /// The Terminal is currently playing the animation.
    Running,
    /// The Terminal has paused the animation.
    Paused,
    /// The Terminal has stopped the animation.
    Stopped,
}

/// The `Terminal` struct handles the display of the animation in the terminal and
/// user input events.
pub struct Terminal {
    /// The foreground color for the terminal display.
    fg_color: Color,
    /// The background color for the terminal display.
    bg_color: Color,
    /// The title of the terminal window.
    title: String,
    /// The current playback state of the Terminal.
    state: State,
    /// The channel for receiving the processed frames from the media processing thread.
    rx_buffer: Receiver<Option<StringInfo>>,
    /// The channel for sending control events to the media processing thread.
    tx_control: Sender<MediaControl>,
    /// Whether to use grayscale colors.
    use_grayscale: bool,
}

impl Terminal {
    /// Constructs a new Terminal with the specified title.
    ///
    /// # Arguments
    ///
    /// * `title` - The title for the terminal window.
    /// * `use_grayscale` - Whether to use grayscale colors.
    /// * `rx_buffer` - The channel for receiving the processed frames from the media processing
    ///   thread.
    /// * `tx_control` - The channel for sending control events to the media processing thread.
    /// * `barrier` - The barrier for synchronizing the media processing thread and the terminal
    ///   thread.
    pub fn new(
        title: String,
        use_grayscale: bool,
        rx_buffer: Receiver<Option<StringInfo>>,
        tx_control: Sender<MediaControl>,
    ) -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
            state: State::Running,
            rx_buffer,
            tx_control,
            use_grayscale,
        }
    }

    /// The main loop of the Terminal that runs the animation, handles user input,
    /// and manages the playback state.
    ///
    /// # Arguments
    ///
    /// * `barrier` - A reference to the `Barrier` used to synchronize the start of the animation.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations or
    /// communication with the pipeline.
    pub fn run(&mut self, barrier: std::sync::Arc<std::sync::Barrier>) -> Result<(), MyError> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(&self.title))?;
        terminal::enable_raw_mode()?;

        // Clear screen
        self.clear()?;

        // Initialize terminal size and pass terminal size to pipeline
        let (width, height) = terminal::size()?;
        self.send_control(MediaControl::Resize(width, height))?;

        barrier.wait();
        // Begin drawing and event loop
        while self.state != State::Stopped {
            // Poll and handle events
            if event::poll(Duration::from_millis(0))? {
                let ev = event::read()?;
                self.handle_event(ev)?;
            }

            // Fetch next frame or detect pipeline shutdown.
            match self.rx_buffer.try_recv() {
                Ok(Some(s)) => {
                    self.draw(&s)?;
                }
                Ok(None) => {
                    // Pipeline heartbeat: nothing to draw.
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    // No frame available right now.
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    // Producer dropped: exit gracefully.
                    self.state = State::Stopped;
                }
            }
        }

        self.cleanup()?;
        Ok(())
    }

    /// Clears the terminal screen and sets the initial terminal state.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn clear(&self) -> IOResult<()> {
        execute!(
            stdout(),
            Clear(ClearType::All),
            Hide,
            SetForegroundColor(self.fg_color),
            SetBackgroundColor(self.bg_color),
            MoveTo(0, 0),
        )?;
        stdout().flush()?;
        Ok(())
    }

    /// Restores the terminal to its original state after the animation has stopped.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn cleanup(&self) -> IOResult<()> {
        // Restore terminal state
        execute!(
            stdout(),
            ResetColor,
            Clear(ClearType::All),
            Show,
            LeaveAlternateScreen
        )?;
        terminal::disable_raw_mode()?;
        Ok(())
    }

    /// Draws the current frame of the animation in the terminal.
    ///
    /// This function takes a reference to a `StringInfo` tuple containing the string representation
    /// of the current frame and its associated RGB data. It either prints the string as-is (in grayscale)
    /// or generates a colored string based on the RGB data and then prints it to the terminal.
    ///
    /// # Arguments
    ///
    /// * `string_info` - A reference to the `StringInfo` tuple containing the string representation
    ///                   of the current frame and its associated RGB data.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn draw(&self, (string, rgb_data): &StringInfo) -> IOResult<()> {
        let print_string = |string: &str| {
            let mut out = stdout();
            execute!(out, MoveTo(0, 0), Print(string), MoveTo(0, 0))?;
            out.flush()?;
            Ok(())
        };

        if self.use_grayscale {
            print_string(string)
        } else {
            let mut colored_string = String::with_capacity(string.len() * 10);
            for (c, rgb) in string.chars().zip(rgb_data.chunks(3)) {
                let color = Color::Rgb {
                    r: rgb[0],
                    g: rgb[1],
                    b: rgb[2],
                };
                colored_string.push_str(&format!("{}", c.stylize().with(color)));
            }
            print_string(&colored_string)
        }
    }

    /// Handles user input events such as pausing/continuing, resizing, and
    /// changing character maps.
    ///
    /// # Arguments
    ///
    /// * `event` - The event received from the terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn handle_event(&mut self, event: Event) -> IOResult<()> {
        match event {
            // Quit
            Event::Key(KeyEvent {
                code: KeyCode::Char('q') | KeyCode::Char('Q'),
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('c') | KeyCode::Char('C'),
                modifiers: event::KeyModifiers::CONTROL,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Esc, ..
            }) => {
                self.state = State::Stopped;
                self.send_control(MediaControl::Exit)?;
            }

            // Pause/Continue
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                self.send_control(MediaControl::PauseContinue)?;
                self.state = match self.state {
                    State::Running => State::Paused,
                    State::Paused => State::Running,
                    State::Stopped => State::Stopped,
                };
            }

            // Resize
            Event::Resize(width, height) => {
                self.send_control(MediaControl::Resize(width, height))?;
                // Drain buffer
                while self
                    .rx_buffer
                    .recv_timeout(Duration::from_millis(1))
                    .is_ok()
                { /* Do nothing */ }
            }

            // Change character map
            Event::Key(KeyEvent {
                code: KeyCode::Char(digit),
                ..
            }) if digit.is_ascii_digit() => {
                self.send_control(MediaControl::SetCharMap(digit.to_digit(10).unwrap_or_else(
                    || panic!("{error}: {digit:?}", error = ERROR_PARSE_DIGIT_FAILED),
                )))?;
            }

            // Toggle grayscale mode
            Event::Key(KeyEvent {
                code: KeyCode::Char('g') | KeyCode::Char('G'),
                ..
            }) => {
                self.use_grayscale = !self.use_grayscale;
                self.send_control(MediaControl::SetGrayscale(self.use_grayscale))?;
            }

            // Mute/unmute
            Event::Key(KeyEvent {
                code: KeyCode::Char('m') | KeyCode::Char('M'),
                ..
            }) => {
                self.send_control(MediaControl::MuteUnmute)?;
            }

            _ => {}
        }
        Ok(())
    }

    /// Sends a control command to the media processing thread.
    ///
    /// # Arguments
    ///
    /// * `control` - The control command to send.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    /// or communication with the pipeline.
    fn send_control(&self, control: MediaControl) -> Result<(), MyError> {
        self.tx_control
            .send(control)
            .map_err(|e| MyError::Terminal(format!("{error}: {e:?}", error = ERROR_CHANNEL, e = e)))
    }
}
