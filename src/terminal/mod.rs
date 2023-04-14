//! The `terminal` module provides functionality for displaying an animation in
//! the terminal and handling user input events such as pausing/continuing,
//! resizing, and changing character maps.
use crate::common::errors::{MyError, ERROR_CHANNEL, ERROR_PARSE_DIGIT_FAILED};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
    Result as CTResult,
};

use std::{
    io::{stdout, Write},
    sync::{mpsc::Receiver, mpsc::Sender},
};

use std::time::Duration;

use crate::runner::Control;

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
    rx_buffer: Receiver<Option::<String>>,
    /// The channel for sending control events to the media processing thread.
    tx_control: Sender<Control>,
}
impl Terminal {
    /// Constructs a new Terminal with the specified title.
    ///
    /// # Arguments
    ///
    /// * `title` - The title for the terminal window.
    pub fn new(title: String, rx_buffer: Receiver<Option<String>>, tx_control: Sender<Control>) -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
            state: State::Running,
            rx_buffer,
            tx_control,
        }
    }

    /// Clears the terminal screen and sets the initial terminal state.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn clear(&self) -> CTResult<()> {
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
    fn cleanup(&self) -> CTResult<()> {
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
    /// # Arguments
    ///
    /// * `string` - The string representation of the current frame to be displayed.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn draw(&self, string: &String) -> CTResult<()> {
        execute!(stdout(), MoveTo(0, 0), Print(string), MoveTo(0, 0),)?;
        // Flush output
        stdout().flush()?;
        Ok(())
    }

    /// Handles user input events such as pausing/continuing, resizing, and
    /// changing character maps.
    ///
    /// # Arguments
    ///
    /// * `event` - The event received from the terminal.
    /// * `commands_buffer` - The shared buffer for sending control commands.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations.
    fn handle_event(&mut self, event: Event) -> CTResult<()> {
        match event {
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
                self.send_control(Control::Exit)?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                self.send_control(Control::PauseContinue)?;
                self.state = match self.state {
                    State::Running => State::Paused,
                    State::Paused => State::Running,
                    State::Stopped => State::Stopped,
                };
            }
            Event::Resize(width, height) => {
                self.send_control(Control::Resize(width, height))?;
                // Drain buffer
                while let Ok(_) = self.rx_buffer.recv_timeout(Duration::from_millis(1)){};
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(digit),
                ..
            }) if digit.is_ascii_digit() => {
                self.send_control(Control::SetCharMap(digit.to_digit(10).unwrap_or_else(
                    || panic!("{error}: {digit:?}", error = ERROR_PARSE_DIGIT_FAILED),
                )))?;
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
    fn send_control(&self, control: Control) -> Result<(), MyError> {
        self.tx_control
            .send(control)
            .map_err(|e| MyError::Terminal(format!("{error}: {e:?}", error = ERROR_CHANNEL, e = e)))
    }

    /// The main loop of the Terminal that runs the animation, handles user input,
    /// and manages the playback state.
    ///
    /// # Arguments
    ///
    /// * `string_buffer` - The shared buffer containing processed frames as strings.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations or
    /// communication with the pipeline.
    pub fn run(&mut self) -> Result<(), MyError> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(&self.title))?;
        terminal::enable_raw_mode()?;

        // Clear screen
        self.clear()?;

        // Initialize terminal size and pass terminal size to pipeline
        let (width, height) = terminal::size()?;
        self.send_control(Control::Resize(width, height))?;

        // Begin drawing and event loop
        while self.state != State::Stopped {
            // Poll and handle events
            if event::poll(Duration::from_millis(0))? {
                let ev = event::read()?;
                self.handle_event(ev)?;
            }

            // Wait for next frame to draw
            if let Ok(Some(s)) = self.rx_buffer.recv_timeout(Duration::from_millis(5)){
                self.draw(&s)?;
            };
        }

        self.cleanup()?;
        Ok(())
    }
}
