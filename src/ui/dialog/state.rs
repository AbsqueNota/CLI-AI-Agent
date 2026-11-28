use tokio::sync::oneshot;

use crate::ui::dialog::{
    DialogRequest,
    DialogResult,
    DialogSpec,
};

use crossterm::event::{
    KeyCode,
    KeyEvent,
    KeyModifiers,
};

pub enum DialogAction {
    Continue,
    Complete(DialogResult),
    Cancel,
}

pub struct ActiveDialog {
    pub spec: DialogSpec,
    pub selection: usize,
    pub value: String,

    reply: Option<
        oneshot::Sender<Result<DialogResult, String>>
    >,
}

impl ActiveDialog {
    pub fn new(request: DialogRequest) -> Self {
        let selection = match &request.spec {
            DialogSpec::Select { selected, .. } => *selected,
            _ => 0,
        };

        let value = match &request.spec {
            DialogSpec::Input { value, .. } => value.clone(),
            _ => String::new(),
        };

        Self {
            spec: request.spec,
            selection,
            value,
            reply: Some(request.reply),
        }
    }

    pub fn complete(
        mut self,
        result: DialogResult,
    ) {
        if let Some(reply) = self.reply.take() {
            let _ = reply.send(Ok(result));
        }
    }

    pub fn cancel(
        mut self,
    ) {
        let result = match self.spec {
            DialogSpec::Select { .. } => {
                DialogResult::Selected(None)
            }

            DialogSpec::Input { .. } => {
                DialogResult::Input(None)
            }

            DialogSpec::Confirm { .. } => {
                DialogResult::Confirm(false)
            }
        };

        if let Some(reply) = self.reply.take() {
            let _ = reply.send(Ok(result));
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
    ) -> DialogAction {
        match &self.spec {
            DialogSpec::Select {
                items,
                max_length,
                ..
            } => {
                if items.is_empty() {
                    return DialogAction::Complete(
                        DialogResult::Selected(None)
                    );
                }

                let page = max_length.unwrap_or(8).max(1);

                match key.code {
                    KeyCode::Up => {
                        self.selection =
                            self.selection.saturating_sub(1);
                    }

                    KeyCode::Down => {
                        self.selection = (self.selection + 1)
                            .min(items.len() - 1);
                    }

                    KeyCode::Home => {
                        self.selection = 0;
                    }

                    KeyCode::End => {
                        self.selection = items.len() - 1;
                    }

                    KeyCode::PageUp => {
                        self.selection =
                            self.selection.saturating_sub(page);
                    }

                    KeyCode::PageDown => {
                        self.selection =
                            (self.selection + page)
                                .min(items.len() - 1);
                    }

                    KeyCode::Enter => {
                        return DialogAction::Complete(
                            DialogResult::Selected(
                                Some(self.selection)
                            )
                        );
                    }

                    KeyCode::Esc => {
                        return DialogAction::Cancel;
                    }

                    _ => {}
                }
            }

            DialogSpec::Input {
                allow_empty,
                ..
            } => {
                let ctrl = key.modifiers
                    .contains(KeyModifiers::CONTROL);

                let alt = key.modifiers
                    .contains(KeyModifiers::ALT);

                match key.code {
                    KeyCode::Char('u') if ctrl => {
                        self.value.clear();
                    }

                    KeyCode::Char(c) if !ctrl && !alt => {
                        self.value.push(c);
                    }

                    KeyCode::Backspace => {
                        self.value.pop();
                    }

                    KeyCode::Enter => {
                        if *allow_empty || !self.value.is_empty() {
                            return DialogAction::Complete(
                                DialogResult::Input(
                                    Some(self.value.clone())
                                )
                            );
                        }
                    }

                    KeyCode::Esc => {
                        return DialogAction::Cancel;
                    }

                    _ => {}
                }
            }

            DialogSpec::Confirm { default, .. } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        return DialogAction::Complete(
                            DialogResult::Confirm(true)
                        );
                    }

                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        return DialogAction::Complete(
                            DialogResult::Confirm(false)
                        );
                    }

                    KeyCode::Enter => {
                        return DialogAction::Complete(
                            DialogResult::Confirm(*default)
                        );
                    }

                    KeyCode::Esc => {
                        return DialogAction::Cancel;
                    }

                    _ => {}
                }
            }
        }

        DialogAction::Continue
    }
}