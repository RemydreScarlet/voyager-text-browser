use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::app::App;
use crate::types::Mode;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    // URL Bar
    f.render_widget(
        Paragraph::new(app.current_url.as_str())
            .block(Block::default().borders(Borders::ALL).title(" Voyager URL ")),
        chunks[0]
    );

    // Main Content
    f.render_widget(
        Paragraph::new(app.render_content())
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .scroll((app.scroll, 0)),
        chunks[1]
    );

    // Status Bar
    let status_text = match app.mode {
        Mode::Command => format!(":{}", app.command_buffer),
        Mode::Normal => format!(
            " {} | Link [{}]: {}",
            app.status,
            app.selected_link_idx,
            if app.links.is_empty() { "" } else { &app.links[app.selected_link_idx].url }
        ),
    };
    f.render_widget(
        Paragraph::new(status_text).style(Style::default().bg(Color::White).fg(Color::Black)),
        chunks[2]
    );

    // Image Popup
    if let Some(ref aa) = app.image_preview {
        let area = centered_rect(80, 80, f.size());
        f.render_widget(Clear, area);
        let aa_lines: Vec<Line> = aa.iter().map(|s| Line::from(s.clone())).collect();
        f.render_widget(
            Paragraph::new(aa_lines)
                .block(Block::default().borders(Borders::ALL).title(" Image AA Preview "))
                .style(Style::default().bg(Color::Black)),
            area
        );
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
