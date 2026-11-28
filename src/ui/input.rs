use crossterm::event::{
    KeyCode,
    KeyEvent,
    KeyModifiers,
};

use crate::app::events::AppCommand;

#[derive(Debug, Default)]
pub struct Input {
    value: String,
    cursor: usize,
    focused: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            focused: true,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.value)
    }

    fn byte_index(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }

    fn insert(&mut self, character: char) {
        let index = self.byte_index(self.cursor);

        self.value.insert(index, character);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);

        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        if self.cursor >= self.value.chars().count() {
            return;
        }

        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);

        self.value.replace_range(start..end, "");
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
    ) -> Option<AppCommand> {
        if !self.focused {
            return None;
        }

        let ctrl = key.modifiers.contains(
            KeyModifiers::CONTROL
        );

        let alt = key.modifiers.contains(
            KeyModifiers::ALT
        );

        if ctrl && key.code == KeyCode::Char('c') {
            return Some(AppCommand::Interrupt);
        }

        if ctrl && key.code == KeyCode::Char('q') {
            return Some(AppCommand::Exit);
        }

        match key.code {
            KeyCode::Char('u') if ctrl => {
                self.clear();
            }

            KeyCode::Char(character)
                if !ctrl && !alt =>
            {
                self.insert(character);
            }

            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            }

            KeyCode::Right => {
                self.cursor = (self.cursor + 1)
                    .min(self.value.chars().count());
            }

            KeyCode::Home => {
                self.cursor = 0;
            }

            KeyCode::End => {
                self.cursor = self.value.chars().count();
            }

            KeyCode::Backspace => {
                self.backspace();
            }

            KeyCode::Delete => {
                self.delete();
            }

            KeyCode::Enter => {
                let value = self.take();
                let value = value.trim();

                if !value.is_empty() {
                    return Some(AppCommand::Submit(
                        value.to_string()
                    ));
                }
            }

            KeyCode::Esc => {
                return Some(AppCommand::Interrupt);
            }

            _ => {}
        }

        None
    }
}