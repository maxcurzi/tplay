use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetSize, SetTitle},
    Result,
};

use std::{
    io::{stdout, Write},
    sync::Condvar,
};

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::Control;

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

    fn poll_events(&self) -> Result<Option<Event>> {
        todo!(); // move event polling logic here
    }

    fn draw(&self, string: &String) {
        todo!(); // move draw logic here
    }

    pub fn run(
        self,
        string_buffer: Arc<Mutex<VecDeque<String>>>,
        condvar: Arc<Condvar>,
        commands_buffer: Arc<Mutex<VecDeque<Control>>>,
    ) -> Result<()> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(self.title))?;
        terminal::enable_raw_mode()?;
        // Clear screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            Hide,
            SetForegroundColor(self.fg_color),
            SetBackgroundColor(self.bg_color),
            MoveTo(0, 0) // Set cursor position
        )?;
        let mut display_string = "".to_string();
        let mut new_frame = false;
        'outer: loop {
            let mut buffer_guard = string_buffer.lock().unwrap();
            (buffer_guard, _) = condvar
                .wait_timeout(buffer_guard, Duration::from_millis(100))
                .unwrap();
            if let Some(s) = buffer_guard.pop_front() {
                display_string = s;
                new_frame = true;
            }
            drop(buffer_guard);

            if new_frame {
                execute!(
                    stdout(),
                    SetForegroundColor(self.fg_color),
                    SetBackgroundColor(self.bg_color),
                    Print(display_string.clone()),
                    ResetColor,
                    MoveTo(0, 0) // Set cursor position
                )?;
                // Flush output
                stdout().flush()?;
                new_frame = false;
            }

            // Wait for user input
            match event::poll(Duration::from_millis(0))? {
                true => {
                    let ev = event::read()?;
                    match ev {
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Char('Q'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: event::KeyModifiers::CONTROL,
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Char('C'),
                            modifiers: event::KeyModifiers::CONTROL,
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Esc, ..
                        }) => {
                            let mut control_guard = commands_buffer.lock().unwrap();
                            control_guard.push_back(Control::Exit);
                            break 'outer;
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Char(' '),
                            ..
                        }) => {
                            let mut control_guard = commands_buffer.lock().unwrap();
                            control_guard.push_back(Control::PauseContinue);
                        }
                        Event::Resize(_, _) => {
                            // Drain buffer
                            let mut buffer_guard = string_buffer.lock().unwrap();
                            buffer_guard.clear();
                            // Clear screen
                            execute!(
                                stdout(),
                                Clear(ClearType::All),
                                Hide,
                                SetForegroundColor(self.fg_color),
                                SetBackgroundColor(self.bg_color),
                                MoveTo(0, 0) // Set cursor position
                            )?;
                            // Resize terminal
                            let (width, height) = terminal::size().unwrap();
                            execute!(stdout(), SetSize(width, height))?;
                            let mut control_guard = commands_buffer.lock().unwrap();
                            control_guard.push_back(Control::Refresh);
                            // Reset vars
                            display_string = "".to_string();
                        }
                        _ => {}
                    }
                }
                false => std::thread::sleep(Duration::from_millis(10)),
            }
        }

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
}
