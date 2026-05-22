mod app;
mod types;
mod ui;

use crate::app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = App::new("https://www.rust-lang.org");
    app.data.borrow_mut().status = "Loading...".to_string();

    'main: loop {
        app.servo.spin_event_loop();
        app.process_extraction();

        let running = {
            terminal.draw(|f| {
                let d = app.data.borrow();
                ui::draw(f, &d);
            })?;

            if crossterm::event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = crossterm::event::read()? {
                    if !app.handle_key(key) {
                        break 'main;
                    }
                }
            }
            true
        };

        if !running {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
