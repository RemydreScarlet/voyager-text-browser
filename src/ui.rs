use crate::app::{render_content, DisplayData};
use crate::types::Mode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, data: &DisplayData) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    f.render_widget(
        Paragraph::new(data.current_url.as_str())
            .block(Block::default().borders(Borders::ALL).title(" Voyager URL ")),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(render_content(data))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .scroll((data.scroll, 0)),
        chunks[1],
    );

    let status_text = match data.mode {
        Mode::Command => format!(":{}", data.command_buffer),
        Mode::Normal => format!(
            " {} | Link [{}]: {}",
            data.status,
            data.selected_link_idx,
            if data.links.is_empty() {
                ""
            } else {
                &data.links[data.selected_link_idx].url
            }
        ),
    };
    f.render_widget(
        Paragraph::new(status_text).style(Style::default().bg(Color::White).fg(Color::Black)),
        chunks[2],
    );

    if let Some(ref aa) = data.image_preview {
        let area = centered_rect(80, 80, f.size());
        f.render_widget(Clear, area);
        let aa_lines: Vec<Line> = aa.iter().map(|s| Line::from(s.clone())).collect();
        f.render_widget(
            Paragraph::new(aa_lines)
                .block(Block::default().borders(Borders::ALL).title(" Image AA Preview "))
                .style(Style::default().bg(Color::Black)),
            area,
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
