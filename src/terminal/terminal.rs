mod cli;
mod pipeline;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
    Result,
};
use std::io::{stdout, Write}; // Add this line

use std::time::Duration;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() -> Result<()> {
    // Initialize terminal
    execute!(stdout(), EnterAlternateScreen, SetTitle("My App"))?;
    terminal::enable_raw_mode()?;

    // Define colors
    let red = Color::Red;
    let green = Color::Green;

    // Get initial terminal size
    let (width, height) = terminal::size()?;
    // let (old_width, old_height) = (width, height);

    // Create atomic boolean to track resize events
    let resized = Arc::new(AtomicBool::new(false));
    let resized_clone = resized.clone();

    // Handle resize events in separate thread
    std::thread::spawn(move || loop {
        if let Ok(Event::Resize(_, _)) = event::read() {
            resized_clone.store(true, Ordering::Relaxed);
        }
    });

    loop {
        //!resized.load(Ordering::Relaxed) {
        // Clear screen
        execute!(
            stdout(),
            Clear(ClearType::All),
            Hide,
            SetForegroundColor(red),
            SetBackgroundColor(green),
            MoveTo(0, 0) // Set cursor position
        )?;
        let (width, height) = terminal::size()?;

        // Print colored text to screen
        execute!(
            stdout(),
            SetForegroundColor(Color::White),
            SetBackgroundColor(Color::Reset),
            Print(format!(
                "Terminal size: {} rows x {} columns\n",
                height, width
            )),
            SetForegroundColor(red),
            SetBackgroundColor(green),
            Print(format!("Hello, World! ({}x{})", width, height)),
            ResetColor,
        )?;

        // Flush output
        stdout().flush()?;

        // Wait for user input
        match event::poll(Duration::from_millis(10))? {
            true => {
                break;
                std::thread::sleep(Duration::from_millis(10));
            }
            false => {
                std::thread::sleep(Duration::from_millis(10));
            }
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
