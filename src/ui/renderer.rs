use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::ui::{
    state::{AgentPhase, ChatMessage, MessageRole, UiState},
    theme::AppTheme,
};

fn message_lines(
    message: &ChatMessage,
    lines: &mut Vec<Line<'static>>,
    theme: &AppTheme,
) {
    let (label, label_color) = match message.role {
        MessageRole::User      => ("You",       theme.palette.main),
        MessageRole::Assistant => ("Assistant", theme.palette.info),
        MessageRole::System    => ("System",    theme.palette.muted),
        MessageRole::Tool      => ("Tool",      theme.palette.muted),
        MessageRole::Info      => ("Info",      theme.palette.info),
        MessageRole::Warning   => ("Warning",   theme.palette.warning),
        MessageRole::Error     => ("Error",     theme.palette.error),
    };

    for (index, content) in message.content.split('\n').enumerate() {
        let mut spans = Vec::new();

        if index == 0 {
            spans.push(Span::styled(
                format!("{label}: "),
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("  "));
        }

        spans.push(Span::styled(
            content.to_string(),
            Style::default().fg(theme.palette.default),
        ));

        lines.push(Line::from(spans));
    }

    lines.push(Line::default());
}

pub fn render(
    frame: &mut Frame,
    state: &UiState,
    theme: &AppTheme,
) {
    let palette = &theme.palette;

    let main = palette.main;
    let default = palette.default;
    let muted = palette.muted;
    let info = palette.info;

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

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "✦ Local AI Assistant ✦",
            Style::default()
                .fg(main)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, areas[0]);

    // Chat history

    let mut lines = Vec::new();

    for message in &state.messages {
        message_lines(message, &mut lines, theme);
    }
    
    for notice in &state.notices {
        message_lines(notice, &mut lines, theme);
    }
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
                    agent.reasoning.clone(),
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
                    agent.answer.clone(),
                    Style::default().fg(default),
                ),
            ]));
        }

        lines.push(Line::default());
    }

    let history_block = Block::default()
        .title(" Chat ")
        .borders(Borders::ALL);

    let inner = history_block.inner(areas[1]);

    let content_height: u16 = if inner.width == 0 {
        0
    } else {
        let width = inner.width as usize;
        lines
            .iter()
            .map(|line| {
                let w = line.width();
                if w == 0 { 1 } else { w.div_ceil(width) }
            })
            .sum::<usize>()
            .min(u16::MAX as usize) as u16
    };

    let history = Paragraph::new(lines)
        .wrap(Wrap { trim: false });


    let max_scroll = content_height
        .saturating_sub(inner.height as u16);

    let scroll = max_scroll
        .saturating_sub(state.scroll_offset as u16)
        .min(u16::MAX);

    let history = history
        .scroll((scroll, 0))
        .block(history_block);

    frame.render_widget(history, areas[1]);

    // Input

    let input = Paragraph::new(state.input.value())
        .style(Style::default().fg(default))
        .block(
            Block::default()
                .title(" You ")
                .borders(Borders::ALL),
        );

    frame.render_widget(input, areas[2]);

    // Status

    let status = Paragraph::new(Line::from(vec![
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
    ]));

    frame.render_widget(status, areas[3]);
}
