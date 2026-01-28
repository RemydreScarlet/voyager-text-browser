mod types;
mod app;
mod ui;

use crate::types::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{error::Error, io};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = app::App::new("https://www.rust-lang.org");
    app.fetch_page().await?;

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if app.image_preview.is_some() {
                if key.code == KeyCode::Esc {
                    app.image_preview = None;
                    app.status = "Preview closed".to_string();
                }
                continue;
            }

            match app.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char(':') => { app.mode = Mode::Command; app.command_buffer.clear(); }
                    KeyCode::Char('j') => app.scroll = app.scroll.saturating_add(1),
                    KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::Char('l') | KeyCode::Tab => if !app.links.is_empty() {
                        app.selected_link_idx = (app.selected_link_idx + 1) % app.links.len();
                    }
                    KeyCode::Char('h') => if !app.links.is_empty() {
                        app.selected_link_idx = if app.selected_link_idx == 0 { app.links.len() - 1 } else { app.selected_link_idx - 1 };
                    }
                    KeyCode::Enter => if !app.links.is_empty() {
                        let link = app.links[app.selected_link_idx].clone();
                        if link.link_type == LinkType::Image { app.preview_image(&link.url).await?; }
                        else { app.navigate(link.url).await?; }
                    }
                    _ => {}
                }
                Mode::Command => match key.code {
                    KeyCode::Enter => {
                        let cmd = app.command_buffer.clone();
                        if cmd == "q" { break; }
                        else if cmd.starts_with("url ") { app.navigate(cmd[4..].to_string()).await?; }
                        else if cmd == "back" || cmd == "b" {
                            if let Some(prev) = app.history.pop() {
                                app.future.push(app.current_url.clone());
                                app.current_url = prev;
                                app.fetch_page().await?;
                            }
                        }
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Esc => app.mode = Mode::Normal,
                    KeyCode::Char(c) => app.command_buffer.push(c),
                    KeyCode::Backspace => { app.command_buffer.pop(); }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
