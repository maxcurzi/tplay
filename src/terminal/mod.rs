use crate::common::errors::MyError;

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

pub struct Terminal {
    fg_color: Color,
    bg_color: Color,
    title: String,
}
impl Terminal {
    pub fn new(title: String) -> Self {
        // Define colors
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
        }
    }

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

    fn draw(&self, string: &String) -> CTResult<()> {
        execute!(stdout(), MoveTo(0, 0), Print(string), MoveTo(0, 0),)?;
        // Flush output
        stdout().flush()?;
        Ok(())
    }

    fn handle_event(
        event: Event,
        paused: &mut bool,
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
                let mut control_guard = commands_buffer
                    .lock()
                    .expect("Failed to lock commands buffer");
                control_guard.push_back(Control::Exit);
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(' '),
                ..
            }) => {
                let mut control_guard = commands_buffer
                    .lock()
                    .expect("Failed to lock commands buffer");
                control_guard.push_back(Control::PauseContinue);
                *paused = !*paused;
            }
            Event::Resize(width, height) => {
                let mut control_guard = commands_buffer
                    .lock()
                    .expect("Failed to lock commands buffer");
                control_guard.push_back(Control::Resize(width, height));
            }
            Event::Key(KeyEvent {
                code: KeyCode::Char(digit),
                ..
            }) if digit.is_ascii_digit() => {
                let mut control_guard = commands_buffer
                    .lock()
                    .expect("Failed to lock commands buffer");

                control_guard.push_back(Control::SetCharMap(
                    digit
                        .to_digit(10)
                        .unwrap_or_else(|| panic!("Failed to parse digit:{digit:?}")),
                ));
            }
            _ => {}
        }
        Ok(())
    }

    pub fn run(
        &self,
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
        let mut paused = false;
        'outer: loop {
            if !paused {
                let mut buffer_guard = string_buffer
                    .lock()
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                (buffer_guard, _) = condvar
                    .wait_timeout(buffer_guard, Duration::from_millis(100))
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                let next_frame = buffer_guard.pop_front();
                drop(buffer_guard);

                if let Some(s) = next_frame {
                    self.draw(&s)
                        .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                }
            }

            // Poll events
            if event::poll(Duration::from_millis(5))
                .map_err(|e| MyError::Terminal(format!("{e:?}")))?
            {
                let ev = event::read().map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                Self::handle_event(ev, &mut paused, &commands_buffer)
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?;
                if commands_buffer
                    .lock()
                    .map_err(|e| MyError::Terminal(format!("{e:?}")))?
                    .contains(&Control::Exit)
                {
                    break 'outer;
                }
            }
        }

        self.cleanup()
            .map_err(|e| MyError::Terminal(format!("{e:?}")))
    }
}
