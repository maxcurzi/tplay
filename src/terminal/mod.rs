//! The `terminal` module provides functionality for displaying an animation in
//! the terminal and handling user input events such as pausing/continuing,
//! resizing, and changing character maps.
use crate::common::errors::{MyError, ERROR_LOCK_CMD_BUFFER_FAILED, ERROR_PARSE_DIGIT_FAILED};

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
    sync::Condvar,
};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
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
}
impl Terminal {
    /// Constructs a new Terminal with the specified title.
    ///
    /// # Arguments
    ///
    /// * `title` - The title for the terminal window.
    pub fn new(title: String) -> Self {
        // Define colors
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
            state: State::Running,
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
    fn handle_event(
        &mut self,
        event: Event,
        commands_buffer: &Arc<Mutex<VecDeque<Control>>>,
    ) -> CTResult<()> {
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
                let mut control_guard = commands_buffer.lock().expect(ERROR_LOCK_CMD_BUFFER_FAILED);
                control_guard.push_back(Control::Exit);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                let mut control_guard = commands_buffer.lock().expect(ERROR_LOCK_CMD_BUFFER_FAILED);
                control_guard.push_back(Control::PauseContinue);
                self.state = match self.state {
                    State::Running => State::Paused,
                    State::Paused => State::Running,
                    State::Stopped => State::Stopped,
                };
            }
            Event::Resize(width, height) => {
                let mut control_guard = commands_buffer.lock().expect(ERROR_LOCK_CMD_BUFFER_FAILED);
                control_guard.push_back(Control::Resize(width, height));
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(digit),
                ..
            }) if digit.is_ascii_digit() => {
                let mut control_guard = commands_buffer.lock().expect(ERROR_LOCK_CMD_BUFFER_FAILED);

                control_guard.push_back(Control::SetCharMap(digit.to_digit(10).unwrap_or_else(
                    || panic!("{error}: {digit:?}", error = ERROR_PARSE_DIGIT_FAILED),
                )));
            }
            _ => {}
        }
        Ok(())
    }

    /// The main loop of the Terminal that runs the animation, handles user input,
    /// and manages the playback state.
    ///
    /// # Arguments
    ///
    /// * `string_buffer` - The shared buffer containing processed frames as strings.
    /// * `condvar` - The condition variable to notify that a new frame is ready.
    /// * `commands_buffer` - The shared buffer for sending control commands.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with the terminal operations or
    /// synchronization primitives.
    pub fn run(
        &mut self,
        string_buffer: Arc<Mutex<VecDeque<String>>>,
        condvar: Arc<Condvar>,
        commands_buffer: Arc<Mutex<VecDeque<Control>>>,
    ) -> Result<(), MyError> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(&self.title))
            .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
        terminal::enable_raw_mode().map_err(|e| MyError::Terminal(format!("{e:?}")))?;
        let (width, height) = terminal::size().map_err(|e| MyError::Terminal(format!("{e:?}")))?;

        // Clear screen
        self.clear()
            .map_err(|e| MyError::Terminal(format!("{e:?}")))?;

        // Initialize terminal size and pass terminal size to pipeline
        let mut control_guard = commands_buffer
            .lock()
            .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
        control_guard.push_back(Control::Resize(width, height));
        drop(control_guard);

        // Begin drawing and event loop
        while self.state != State::Stopped {
            // Poll and handle events
            if event::poll(Duration::from_millis(1))
                .map_err(|e| MyError::Terminal(format!("{e:?}")))?
            {
                let ev = event::read().map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                self.handle_event(ev, &commands_buffer)
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
            }

            // Wait for next frame to draw
            let mut buffer_guard = string_buffer
                .lock()
                .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
            (buffer_guard, _) = condvar
                .wait_timeout(buffer_guard, Duration::from_millis(5))
                .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
            let next_frame = buffer_guard.pop_front();
            drop(buffer_guard);

            if let Some(s) = next_frame {
                self.draw(&s)
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
            }
        }

        self.cleanup()
            .map_err(|e| MyError::Terminal(format!("{e:?}")))
    }
}
