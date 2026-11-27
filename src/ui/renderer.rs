
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::ui::{
    state::{MessageRole, UiState},
    theme::AppTheme,
};

fn to_ratatui_color(color: colored::Color) -> Color {
    match color {
        colored::Color::TrueColor { r, g, b } => {
            Color::Rgb(r, g, b)
        }
        _ => Color::Reset,
    }
}

pub fn render(
    frame: &mut Frame,
    state: &UiState,
    theme: &AppTheme,
) {
    let palette = &theme.palette;

    let main = to_ratatui_color(palette.main);
    let default = to_ratatui_color(palette.default);
    let muted = to_ratatui_color(palette.muted);
    let info = to_ratatui_color(palette.info);

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    // Header

    let header = Paragraph::new(
        Line::from(vec![
            Span::styled(
                "✦ Local AI Assistant ✦",
                Style::default()
                    .fg(main)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    )
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, areas[0]);

    // Chat history

    let mut lines = Vec::new();

    for message in &state.messages {
        let (label, color) = match message.role {
            MessageRole::User => ("You", info),
            MessageRole::Assistant => ("Assistant", main),
            MessageRole::System => ("System", muted),
            MessageRole::Error => ("Error", Color::Red),
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{label}: "),
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                message.content.as_str(),
                Style::default().fg(default),
            ),
        ]));

        lines.push(Line::default());
    }

    let history = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((state.scroll_offset, 0))
        .block(
            Block::default()
                .title(" Chat ")
                .borders(Borders::ALL),
        );

    frame.render_widget(history, areas[1]);

    // Input

    let input = Paragraph::new(state.input.as_str())
        .style(Style::default().fg(default))
        .block(
            Block::default()
                .title(" You ")
                .borders(Borders::ALL),
        );

    frame.render_widget(input, areas[2]);

    // Status

    let status = Paragraph::new(
        Line::from(vec![
            Span::styled(
                format!(" {} ", state.status),
                Style::default().fg(info),
            ),
            Span::styled(
                format!("Agents: {}", state.active_agents()),
                Style::default().fg(muted),
            ),
        ])
    );

    frame.render_widget(status, areas[3]);
}
