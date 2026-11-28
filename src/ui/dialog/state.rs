
use tokio::sync::oneshot;

use crossterm::event::{
    KeyCode,
    KeyEvent,
};

use crate::ui::{
    dialog::{
        DialogRequest,
        DialogResult,
        DialogSpec,
    },
    input::{
        Input,
        InputEdit,
    },
};

pub enum DialogAction {
    Continue,
    Complete(DialogResult),
    Cancel,
}

pub struct ActiveDialog {
    pub spec: DialogSpec,
    pub selection: usize,

    input: Input,

    reply: Option<
        oneshot::Sender<Result<DialogResult, String>>
    >,
}

impl ActiveDialog {
    pub fn new(request: DialogRequest) -> Self {
        let selection = match &request.spec {
            DialogSpec::Select {
                selected,
                items,
                ..
            } => {
                if items.is_empty() {
                    0
                } else {
                    (*selected).min(items.len() - 1)
                }
            }
            _ => 0,
        };

        let mut input = Input::new();

        if let DialogSpec::Input { value, .. } = &request.spec {
            input.set_value(value.clone());
        }

        Self {
            spec: request.spec,
            selection,
            input,
            reply: Some(request.reply),
        }
    }

    pub fn value(&self) -> &str {
        self.input.value()
    }

    pub fn cursor(&self) -> usize {
        self.input.cursor()
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
        if let Some(reply) = self.reply.take() {
            let _ = reply.send(Ok(DialogResult::Cancelled));
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

                let page = max_length
                    .unwrap_or(8)
                    .max(1);

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
                        self.selection = (self.selection + page)
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
                match self.input.handle_edit_key(key) {
                    InputEdit::Submit => {
                        if *allow_empty
                            || !self.input.value().is_empty()
                        {
                            return DialogAction::Complete(
                                DialogResult::Input(
                                    Some(self.input.value().to_string())
                                )
                            );
                        }
                    }

                    InputEdit::Cancel => {
                        return DialogAction::Cancel;
                    }

                    InputEdit::Changed
                    | InputEdit::Ignored => {}
                }
            }

            DialogSpec::Confirm { default, .. } => {
                match key.code {
                    KeyCode::Char('y')
                    | KeyCode::Char('Y') => {
                        return DialogAction::Complete(
                            DialogResult::Confirm(true)
                        );
                    }

                    KeyCode::Char('n')
                    | KeyCode::Char('N') => {
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

#[derive(Default)]
pub struct DialogState {
    active: Option<ActiveDialog>,
}

impl DialogState {
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<&ActiveDialog> {
        self.active.as_ref()
    }

    pub fn open(
        &mut self,
        request: DialogRequest,
    ) -> Result<(), DialogRequest> {
        if self.active.is_some() {
            return Err(request);
        }

        self.active = Some(ActiveDialog::new(request));

        Ok(())
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
    ) -> bool {
        let Some(active) = self.active.as_mut() else {
            return false;
        };

        match active.handle_key(key) {
            DialogAction::Continue => {}

            DialogAction::Complete(result) => {
                if let Some(active) = self.active.take() {
                    active.complete(result);
                }
            }

            DialogAction::Cancel => {
                if let Some(active) = self.active.take() {
                    active.cancel();
                }
            }
        }

        true
    }
}
