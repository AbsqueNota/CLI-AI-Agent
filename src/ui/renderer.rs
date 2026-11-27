
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::ui::{
    state::{AgentPhase, MessageRole, UiState},
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

    let history_block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL);

    let inner = history_block.inner(areas[1]);

    let max_scroll = lines
        .len()
        .saturating_sub(inner.height as usize);

    let scroll = max_scroll
        .saturating_sub(state.scroll_offset as usize)
        .min(u16::MAX as usize) as u16;

    let history = Paragraph::new(lines.clone())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(history_block);

    frame.render_widget(history, areas[1]);

    // Active agents

    for agent in state.agents.values() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("[{}]", agent.model_id),
                Style::default()
                    .fg(main)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                match agent.phase {
                    AgentPhase::Starting => " starting",
                    AgentPhase::Reasoning => " thinking",
                    AgentPhase::Answering => " answering",
                    AgentPhase::Finished => " finished",
                    AgentPhase::Failed => " failed",
                },
                Style::default().fg(muted),
            ),
        ]));

        if !agent.reasoning.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "thinking: ",
                    Style::default().fg(muted),
                ),
                Span::styled(
                    agent.reasoning.as_str(),
                    Style::default().fg(muted),
                ),
            ]));
        }

        if !agent.answer.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    "answer: ",
                    Style::default().fg(info),
                ),
                Span::styled(
                    agent.answer.as_str(),
                    Style::default().fg(default),
                ),
            ]));
        }

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

    // Input placeholder

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
        Span::styled(
            "  Enter: send  Ctrl+C: exit  PgUp/PgDn: scroll",
            Style::default().fg(muted),
        ),
    ])
);

frame.render_widget(status, areas[3]);
}
