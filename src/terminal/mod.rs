//! The `terminal` module provides functionality for displaying an animation in
//! the terminal and handling user input events such as pausing/continuing,
//! resizing, and changing character maps.
use crate::{
    common::{errors::*, sync::PlaybackClock},
    msg::broker::Control as MediaControl,
    subtitle::SubtitleManager,
    StringInfo,
};
use crossbeam_channel::{Receiver, Sender};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use std::{
    io::{stdout, Result as IOResult, Write},
    sync::{Arc, RwLock},
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

/// Default seek step in seconds for arrow key navigation.
const SEEK_STEP_SECONDS: f64 = 5.0;
/// Larger seek step in seconds for j/l navigation (YouTube-style).
const SEEK_STEP_LARGE_SECONDS: f64 = 10.0;
/// Number of lines reserved at the bottom for subtitles.
const SUBTITLE_LINES: u16 = 1;
/// Default modest size scaling factor (1.0 = ~60% of width)
const SUBTITLE_SIZE: f64 = 1.0;
/// Base percentage of screen width used for subtitles at size 1.0
const SUBTITLE_BASE_WIDTH_PERCENT: f64 = 0.6;

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
    subtitle_text: Option<Arc<RwLock<String>>>,
    subtitles_enabled: bool,
    terminal_width: u16,
    terminal_height: u16,
    local_subtitles: Option<SubtitleManager>,
    playback_clock: Option<Arc<PlaybackClock>>,
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
        subtitle_text: Option<Arc<RwLock<String>>>,
        local_subtitles: Option<SubtitleManager>,
        playback_clock: Option<Arc<PlaybackClock>>,
    ) -> Self {
        Self {
            fg_color: Color::White,
            bg_color: Color::Black,
            title,
            state: State::Running,
            rx_buffer,
            tx_control,
            use_grayscale,
            subtitle_text,
            subtitles_enabled: false,
            terminal_width: 80,
            terminal_height: 24,
            local_subtitles,
            playback_clock,
        }
    }

    pub fn run(&mut self, barrier: std::sync::Arc<std::sync::Barrier>) -> Result<(), MyError> {
        execute!(stdout(), EnterAlternateScreen, SetTitle(&self.title))?;
        terminal::enable_raw_mode()?;

        // Clear screen
        self.clear()?;

        // Initialize terminal size and pass adjusted size to pipeline
        // Only reserve SUBTITLE_LINES at the bottom if subtitles are enabled
        let (width, height) = terminal::size()?;
        self.terminal_width = width;
        self.terminal_height = height;
        let video_height = if self.subtitles_enabled {
            height.saturating_sub(SUBTITLE_LINES)
        } else {
            height
        };
        self.send_control(MediaControl::Resize(width, video_height))?;

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
                    self.draw_subtitle()?;
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
            ResetColor,
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
            let mut colored_string = String::with_capacity(string.len() * 22);
            use std::fmt::Write;
            let mut prev: [u8; 3] = [0, 0, 0];
            let mut first = true;
            for (c, rgb) in string.chars().zip(rgb_data.chunks(3)) {
                if first || rgb[0] != prev[0] || rgb[1] != prev[1] || rgb[2] != prev[2] {
                    let _ = write!(
                        colored_string,
                        "\x1b[38;2;{};{};{}m{}",
                        rgb[0], rgb[1], rgb[2], c
                    );
                    prev = [rgb[0], rgb[1], rgb[2]];
                    first = false;
                } else {
                    colored_string.push(c);
                }
            }
            colored_string.push_str("\x1b[0m");
            print_string(&colored_string)
        }
    }

    fn draw_subtitle(&mut self) -> IOResult<()> {
        if !self.subtitles_enabled {
            return Ok(());
        }

        let subtitle = self.get_current_subtitle();
        if subtitle.is_empty() {
             let subtitle_y = self.terminal_height.saturating_sub(SUBTITLE_LINES);
             let full_width = self.terminal_width as usize;
             let mut out = stdout();
             for y in subtitle_y..self.terminal_height {
                execute!(
                    out,
                    MoveTo(0, y),
                    SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 40 }),
                    Print(" ".repeat(full_width)),
                    ResetColor
                )?;
             }
             out.flush()?;
             return Ok(());
        }

        let full_width = self.terminal_width as usize;
        // Calculate dynamic width based on terminal size and preference
        // 1.0 size = 60% of screen width, scaling with terminal
        // Minimum 40 chars to ensure readability on very small screens
        let target_width = (full_width as f64 * SUBTITLE_BASE_WIDTH_PERCENT * SUBTITLE_SIZE) as usize;
        let max_width = target_width.max(40).min(full_width.saturating_sub(4));
        
        let wrapped_lines = self.wrap_text(&subtitle, max_width);
        
        // Dynamic height: Allow up to 1/4 of screen, minimum 3 lines
        // This ensures the box grows to fit text but doesn't take over whole screen
        let lines_count = wrapped_lines.len();
        let max_visual_lines = (self.terminal_height as usize / 4).max(3);
        let lines_to_draw = lines_count.min(max_visual_lines);
        
        // Calculate Y start based on lines to draw
        let subtitle_y = self.terminal_height.saturating_sub(lines_to_draw as u16);
        let mut out = stdout();

        // Fill background for the dynamic height area
        for i in 0..lines_to_draw {
            let y = subtitle_y + i as u16;
            execute!(
                out,
                MoveTo(0, y),
                SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 40 }),
                Print(" ".repeat(full_width)),
                ResetColor
            )?;
        }

        for (i, line) in wrapped_lines.iter().take(lines_to_draw).enumerate() {
            let y = subtitle_y + i as u16;
            let padding = full_width.saturating_sub(line.len()) / 2;
            let padded_line = format!("{:>width$}", line, width = padding + line.len());
            let fill_spaces = full_width.saturating_sub(padded_line.len());
            
            execute!(
                out,
                MoveTo(0, y),
                SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 40 }),
                SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 255 }),
                SetAttribute(Attribute::Bold),
                Print(&padded_line),
                Print(" ".repeat(fill_spaces)),
                SetAttribute(Attribute::Reset),
                ResetColor
            )?;
        }

        out.flush()?;
        Ok(())
    }

    fn get_current_subtitle(&self) -> String {
        if let Some(ref manager) = self.local_subtitles {
            if let Some(ref clock) = self.playback_clock {
                let pos = clock.get_position();
                if let Some(text) = manager.get_subtitle_at(pos) {
                    return text.to_string();
                }
            }
        }

        if let Some(ref lock) = self.subtitle_text {
            if let Ok(text) = lock.read() {
                return text.clone();
            }
        }

        String::new()
    }

    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
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
                self.terminal_width = width;
                self.terminal_height = height;
                // Only reserve SUBTITLE_LINES at the bottom if subtitles are enabled
                let video_height = if self.subtitles_enabled {
                    height.saturating_sub(SUBTITLE_LINES)
                } else {
                    height
                };
                self.send_control(MediaControl::Resize(width, video_height))?;
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

            // Seek forward (right arrow)
            Event::Key(KeyEvent {
                code: KeyCode::Right,
                ..
            }) => {
                self.send_control(MediaControl::Seek(SEEK_STEP_SECONDS))?;
            }

            // Seek backward (left arrow)
            Event::Key(KeyEvent {
                code: KeyCode::Left,
                ..
            }) => {
                self.send_control(MediaControl::Seek(-SEEK_STEP_SECONDS))?;
            }

            // Seek forward 10s (l key, YouTube-style)
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                ..
            }) => {
                self.send_control(MediaControl::Seek(SEEK_STEP_LARGE_SECONDS))?;
            }

            // Seek backward 10s (j key, YouTube-style)
            Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                ..
            }) => {
                self.send_control(MediaControl::Seek(-SEEK_STEP_LARGE_SECONDS))?;
            }

            // Toggle subtitles on/off (Shift+C)
            Event::Key(KeyEvent {
                code: KeyCode::Char('C'),
                modifiers: KeyModifiers::SHIFT,
                ..
            }) => {
                self.subtitles_enabled = !self.subtitles_enabled;
                
                if let Some(ref mut manager) = self.local_subtitles {
                    manager.set_enabled(self.subtitles_enabled);
                }
                
                self.send_control(MediaControl::ToggleSubtitle)?;
                
                let video_height = if self.subtitles_enabled {
                    self.terminal_height.saturating_sub(SUBTITLE_LINES)
                } else {
                    self.terminal_height
                };
                self.send_control(MediaControl::Resize(self.terminal_width, video_height))?;
                
                if !self.subtitles_enabled {
                    let subtitle_y = self.terminal_height.saturating_sub(SUBTITLE_LINES);
                    let clear_line = " ".repeat(self.terminal_width as usize);
                    let mut out = stdout();
                    for y in subtitle_y..self.terminal_height {
                        let _ = execute!(out, MoveTo(0, y), Print(&clear_line));
                    }
                    let _ = out.flush();
                }
                
                while self
                    .rx_buffer
                    .recv_timeout(Duration::from_millis(1))
                    .is_ok()
                { /* Do nothing */ }
            }

            // Cycle subtitle tracks (lowercase 'c')
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                if let Some(ref mut manager) = self.local_subtitles {
                    manager.cycle_track();
                }
                self.send_control(MediaControl::CycleSubtitle)?;
            }

            // Speed control: decrease by 0.25x
            Event::Key(KeyEvent {
                code: KeyCode::Char('['),
                ..
            }) => {
                self.send_control(MediaControl::AdjustSpeed(-0.25))?;
            }

            // Speed control: increase by 0.25x
            Event::Key(KeyEvent {
                code: KeyCode::Char(']'),
                ..
            }) => {
                self.send_control(MediaControl::AdjustSpeed(0.25))?;
            }

            // Speed control: decrease by 0.1x
            Event::Key(KeyEvent {
                code: KeyCode::Char(','),
                ..
            }) => {
                self.send_control(MediaControl::AdjustSpeed(-0.1))?;
            }

            // Speed control: increase by 0.1x
            Event::Key(KeyEvent {
                code: KeyCode::Char('.'),
                ..
            }) => {
                self.send_control(MediaControl::AdjustSpeed(0.1))?;
            }

            // Speed control: reset to 1.0x
            Event::Key(KeyEvent {
                code: KeyCode::Char('\\'),
                ..
            }) => {
                self.send_control(MediaControl::ResetSpeed)?;
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
