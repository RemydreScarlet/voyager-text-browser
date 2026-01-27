use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kuchiki::traits::*;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};
use std::error::Error;
use std::io;
use url::Url;

#[derive(Debug, PartialEq)]
enum Mode {
    Normal,
    Command,
}

#[derive(Clone)]
struct LinkData {
    text: String,
    url: String,
}

struct App {
    current_url: String,
    content_lines: Vec<Line<'static>>,
    links: Vec<LinkData>,
    selected_link_idx: usize,
    scroll: u16,
    status: String,
    mode: Mode,
    command_buffer: String,
    history: Vec<String>,
    future: Vec<String>,
}

impl App {
    fn new(start_url: &str) -> Self {
        Self {
            current_url: start_url.to_string(),
            content_lines: Vec::new(),
            links: Vec::new(),
            selected_link_idx: 0,
            scroll: 0,
            status: String::from("⚓ Voyager Ready"),
            mode: Mode::Normal,
            command_buffer: String::new(),
            history: Vec::new(),
            future: Vec::new(),
        }
    }

    async fn navigate(&mut self, mut url: String) -> Result<(), Box<dyn Error>> {
        // プロトコルの補完
        if !url.starts_with("http://") && !url.starts_with("https://") {
            url = format!("https://{}", url);
        }
        
        // 履歴の更新
        if !self.current_url.is_empty() {
            self.history.push(self.current_url.clone());
        }
        self.future.clear();
        self.current_url = url;
        self.fetch_page().await
    }

    async fn go_back(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(prev_url) = self.history.pop() {
            self.future.push(self.current_url.clone());
            self.current_url = prev_url;
            self.fetch_page().await?;
        } else {
            self.status = "No back history".to_string();
        }
        Ok(())
    }

    async fn go_forward(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(next_url) = self.future.pop() {
            self.history.push(self.current_url.clone());
            self.current_url = next_url;
            self.fetch_page().await?;
        } else {
            self.status = "No forward history".to_string();
        }
        Ok(())
    }

    async fn fetch_page(&mut self) -> Result<(), Box<dyn Error>> {
        self.status = format!("Fetching {}...", self.current_url);
        let client = reqwest::Client::builder()
            .user_agent("Voyager-Browser/0.1.0")
            .build()?;

        let res = client.get(&self.current_url).send().await?;
        let base_url = Url::parse(&self.current_url)?;
        let html = res.text().await?;

        let document = kuchiki::parse_html().one(html);
        let mut new_lines = Vec::new();
        let mut new_links = Vec::new();

        // 簡易的なパース処理 (h1, h2, p, aを対象)
        for css_match in document.select("h1, h2, p, a").unwrap() {
            let tag = css_match.name.local.to_string();
            let text = css_match.text_contents().trim().to_string();
            if text.is_empty() { continue; }

            match tag.as_str() {
                "h1" => {
                    new_lines.push(Line::from(Span::styled(
                        format!("\n# {}\n", text.to_uppercase()),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )));
                }
                "h2" => {
                    new_lines.push(Line::from(Span::styled(
                        format!("\n## {}\n", text),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )));
                }
                "a" => {
                    let attributes = css_match.attributes.borrow();
                    if let Some(href) = attributes.get("href") {
                        let abs_url = base_url.join(href).map(|u| u.to_string()).unwrap_or(href.to_string());
                        new_links.push(LinkData { text: text.clone(), url: abs_url });
                        new_lines.push(Line::from(Span::styled(
                            format!(" [{}] ", text),
                            Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED),
                        )));
                    }
                }
                _ => {
                    new_lines.push(Line::from(text));
                }
            }
        }

        self.content_lines = new_lines;
        self.links = new_links;
        self.selected_link_idx = 0;
        self.scroll = 0;
        self.status = format!("Loaded: {}", self.current_url);
        Ok(())
    }

    fn render_content(&self) -> Vec<Line<'static>> {
        let mut rendered = Vec::new();
        let mut link_count = 0;

        for line in &self.content_lines {
            let mut spans = Vec::new();
            for span in &line.spans {
                // リンク(Blue)を現在の選択状態に合わせてハイライト
                if span.style.fg == Some(Color::Blue) {
                    let style = if link_count == self.selected_link_idx {
                        Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)
                    } else {
                        span.style
                    };
                    spans.push(Span::styled(span.content.clone(), style));
                    link_count += 1;
                } else {
                    spans.push(span.clone());
                }
            }
            rendered.push(Line::from(spans));
        }
        rendered
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new("https://www.rust-lang.org");
    app.fetch_page().await?;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(f.size());

            // URLバー
            let url_bar = Paragraph::new(app.current_url.as_str())
                .block(Block::default().borders(Borders::ALL).title(" ⚓ Voyager URL "));
            f.render_widget(url_bar, chunks[0]);

            // メインコンテンツ
            let content = Paragraph::new(app.render_content())
                .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
                .scroll((app.scroll, 0))
                .wrap(Wrap { trim: false });
            f.render_widget(content, chunks[1]);

            // ステータス / コマンドバー
            let status_text = match app.mode {
                Mode::Command => format!(":{}", app.command_buffer),
                Mode::Normal => {
                    let link_info = if app.links.is_empty() {
                        "No links".to_string()
                    } else {
                        format!("Link [{}]: {}", app.selected_link_idx, app.links[app.selected_link_idx].url)
                    };
                    format!(" {} | {}", app.status, link_info)
                }
            };
            let status_bar = Paragraph::new(status_text)
                .style(Style::default().bg(Color::White).fg(Color::Black));
            f.render_widget(status_bar, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Normal => match key.code {
                    KeyCode::Char(':') => {
                        app.mode = Mode::Command;
                        app.command_buffer.clear();
                    }
                    KeyCode::Char('j') => app.scroll = app.scroll.saturating_add(1),
                    KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                    KeyCode::Char('l') => {
                        if !app.links.is_empty() {
                            app.selected_link_idx = (app.selected_link_idx + 1) % app.links.len();
                        }
                    }
                    KeyCode::Char('h') => {
                        if !app.links.is_empty() {
                            app.selected_link_idx = if app.selected_link_idx == 0 {
                                app.links.len() - 1
                            } else {
                                app.selected_link_idx - 1
                            };
                        }
                    }
                    KeyCode::Enter => {
                        if !app.links.is_empty() {
                            let url = app.links[app.selected_link_idx].url.clone();
                            app.navigate(url).await?;
                        }
                    }
                    _ => {}
                },
                Mode::Command => match key.code {
                    KeyCode::Enter => {
                        let full_cmd = app.command_buffer.clone();
                        let parts: Vec<&str> = full_cmd.split_whitespace().collect();
                        if !parts.is_empty() {
                            match parts[0] {
                                "q" | "quit" => break,
                                "b" | "back" => app.go_back().await?,
                                "f" | "front" => app.go_forward().await?,
                                "url" => {
                                    if parts.len() > 1 {
                                        app.navigate(parts[1].to_string()).await?;
                                    }
                                }
                                _ => app.status = format!("Unknown: {}", parts[0]),
                            }
                        }
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Esc => app.mode = Mode::Normal,
                    KeyCode::Char(c) => app.command_buffer.push(c),
                    KeyCode::Backspace => { app.command_buffer.pop(); }
                    _ => {}
                },
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
