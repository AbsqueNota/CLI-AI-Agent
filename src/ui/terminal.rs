pub mod event_reader;
use std::io::{self, Stdout};

use crossterm::{
    execute,
    event::{EnableMouseCapture, DisableMouseCapture},
    terminal::{
        enable_raw_mode,
        disable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};

use ratatui::{
    backend::CrosstermBackend,
    Frame,
    Terminal,
};

use crate::ui::{
    renderer,
    state::UiState,
    theme::AppTheme,
};

type Backend = CrosstermBackend<Stdout>;

pub struct TerminalSession {
    terminal: Option<Terminal<Backend>>,
}

impl TerminalSession {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();

        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture
        ) {
            let _ = disable_raw_mode();
            return Err(error);
        }

        let backend = CrosstermBackend::new(stdout);

        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = execute!(
                    io::stdout(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                );

                let _ = disable_raw_mode();

                return Err(error);
            }
        };

        Ok(Self {
            terminal: Some(terminal),
        })
    }

    pub fn draw(
        &mut self,
        state: &UiState,
        theme: &AppTheme,
    ) -> io::Result<()> {
        self.draw_with(|frame| {
            renderer::render(frame, state, theme);
        })
    }

    pub fn draw_with<F>(&mut self, draw: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        let terminal = self.terminal
            .as_mut()
            .ok_or_else(|| {
                io::Error::other("Terminal is already restored")
            })?;

        terminal.draw(draw)?;

        Ok(())
    }

    pub fn restore(&mut self) -> io::Result<()> {
        let Some(mut terminal) = self.terminal.take() else {
            return Ok(());
        };

        let screen_result = execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );

        let raw_result = disable_raw_mode();

        screen_result.and(raw_result)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}