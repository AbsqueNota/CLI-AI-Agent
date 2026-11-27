use std::fmt::Display;

use anyhow::{bail, Result};
use crossterm::event::{
    self,
    Event,
    KeyCode,
    KeyEventKind,
    KeyModifiers,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        ListState,
        Paragraph,
    },
};

use crate::ui::{
    terminal::TerminalSession,
    theme::AppTheme,
};

fn draw_select(
    frame: &mut Frame<'_>,
    theme: &AppTheme,
    prompt: &str,
    items: &[String],
    selected: usize,
    max_length: Option<usize>,
) {
    let area = frame.area();

    let max_rows = area.height.saturating_sub(6).max(1) as usize;

    let visible_rows = max_length
        .unwrap_or(max_rows)
        .max(1)
        .min(max_rows) as u16;

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(visible_rows.saturating_add(2)),
            Constraint::Length(1),
        ])
        .split(area);

    let main = theme.palette.main;
    let muted = theme.palette.muted;
    let default = theme.palette.default;

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "✦ ",
            Style::default()
                .fg(main)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            prompt,
            Style::default()
                .fg(main)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, areas[0]);

    let list_items: Vec<ListItem<'_>> = items
        .iter()
        .map(|item| {
            ListItem::new(item.as_str())
                .style(Style::default().fg(default))
        })
        .collect();

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_symbol("▸ ")
        .highlight_style(
            Style::default()
                .fg(main)
                .add_modifier(Modifier::BOLD),
        );

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    frame.render_stateful_widget(
        list,
        areas[1],
        &mut list_state,
    );

    let footer = Paragraph::new(
        Line::from(Span::styled(
            " ↑/↓ Navigate   Enter Select   Esc Cancel ",
            Style::default().fg(muted),
        )),
    );

    frame.render_widget(footer, areas[2]);
}

pub fn select<T: Display>(
    theme: &AppTheme,
    prompt: &str,
    items: &[T],
    default: usize,
    max_length: Option<usize>,
) -> Result<Option<usize>> {
    if items.is_empty() {
        return Ok(None);
    }

    let items: Vec<String> = items
        .iter()
        .map(ToString::to_string)
        .collect();

    let mut selected = default.min(items.len() - 1);

    let mut terminal = TerminalSession::new()?;

    loop {
        terminal.draw_with(|frame| {
            draw_select(
                frame,
                theme,
                prompt,
                &items,
                selected,
                max_length,
            );
        })?;

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                match key.code {
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                    }

                    KeyCode::Down => {
                        selected = (selected + 1).min(items.len() - 1);
                    }

                    KeyCode::Home => {
                        selected = 0;
                    }

                    KeyCode::End => {
                        selected = items.len() - 1;
                    }

                    KeyCode::PageUp => {
                        selected = selected.saturating_sub(
                            max_length.unwrap_or(8),
                        );
                    }

                    KeyCode::PageDown => {
                        selected = (selected + max_length.unwrap_or(8))
                            .min(items.len() - 1);
                    }

                    KeyCode::Enter => {
                        terminal.restore()?;
                        return Ok(Some(selected));
                    }

                    KeyCode::Esc => {
                        terminal.restore()?;
                        return Ok(None);
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    }
}

fn draw_input(
    frame: &mut Frame<'_>,
    theme: &AppTheme,
    prompt: &str,
    value: &str,
) {
    let area = frame.area();

    let main = theme.palette.main;
    let default = theme.palette.default;
    let muted = theme.palette.muted;

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let header = Paragraph::new(prompt)
        .style(
            Style::default()
                .fg(main)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, areas[0]);

    let input = Paragraph::new(value)
        .style(Style::default().fg(default))
        .block(
            Block::default()
                .title(" Value ")
                .borders(Borders::ALL),
        );

    frame.render_widget(input, areas[1]);

    let footer = Paragraph::new(
        Line::from(Span::styled(
            " Enter Confirm   Esc Cancel   Ctrl+U Clear ",
            Style::default().fg(muted),
        )),
    );

    frame.render_widget(footer, areas[2]);
}

pub fn input(
    theme: &AppTheme,
    prompt: &str,
    allow_empty: bool,
) -> Result<String> {
    let mut value = String::new();
    let mut terminal = TerminalSession::new()?;

    loop {
        terminal.draw_with(|frame| {
            draw_input(frame, theme, prompt, &value);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                match key.code {
                    KeyCode::Char('u')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        value.clear();
                    }

                    KeyCode::Char(character)
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT) =>
                    {
                        value.push(character);
                    }

                    KeyCode::Backspace => {
                        value.pop();
                    }

                    KeyCode::Enter => {
                        if allow_empty || !value.is_empty() {
                            terminal.restore()?;
                            return Ok(value);
                        }
                    }

                    KeyCode::Esc => {
                        terminal.restore()?;
                        bail!("Input cancelled");
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    }
}

fn draw_confirm(
    frame: &mut Frame<'_>,
    theme: &AppTheme,
    prompt: &str,
    default: bool,
) {
    let area = frame.area();

    let main = theme.palette.main;
    let muted = theme.palette.muted;

    let choice = if default {
        "[Y]es / No"
    } else {
        "Yes / [N]o"
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    let message = Paragraph::new(vec![
        Line::from(Span::styled(
            prompt,
            Style::default().fg(main),
        )),
        Line::from(""),
        Line::from(Span::styled(
            choice,
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ])
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(message, layout[0]);

    let footer = Paragraph::new(
        Line::from(Span::styled(
            " Y Yes   N No   Enter Default   Esc Cancel ",
            Style::default().fg(muted),
        )),
    );

    frame.render_widget(footer, layout[1]);
}

pub fn confirm(
    theme: &AppTheme,
    prompt: &str,
    default: bool,
) -> Result<bool> {
    let mut terminal = TerminalSession::new()?;

    loop {
        terminal.draw_with(|frame| {
            draw_confirm(frame, theme, prompt, default);
        })?;

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        terminal.restore()?;
                        return Ok(true);
                    }

                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        terminal.restore()?;
                        return Ok(false);
                    }

                    KeyCode::Enter => {
                        terminal.restore()?;
                        return Ok(default);
                    }

                    KeyCode::Esc => {
                        terminal.restore()?;
                        return Ok(false);
                    }

                    _ => {}
                }
            }

            _ => {}
        }
    }
}