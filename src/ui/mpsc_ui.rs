use std::{
    io::{self, Write},
    time::Duration,
    collections::VecDeque,
};
use crossterm::event::{
    KeyCode,
    KeyEvent,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::{interval, MissedTickBehavior},
};
use crate::ui::{
    input::{InputEdit},
    state::{
        AgentPhase,
        AgentViewState,
        ChatMessage,
        MessageRole,
        UiState,
    },
    terminal::TerminalSession,
    theme::AppTheme,
};

pub type AgentId = String;

pub const UI_CHANNEL_CAPACITY: usize = 256;
const UI_ACTION_CAPACITY: usize = 32;
const UI_FRAME_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug)]
pub enum UiAction {
    Submit(String),
    Interrupt,
    Exit,
}

#[derive(Debug)]
pub enum UiEvent {
    Key(KeyEvent),

    Resize {
        width: u16,
        height: u16,
    },

    Started {
        agent_id: AgentId,
        model_id: String,
    },

    Reasoning {
        agent_id: AgentId,
        text: String,
    },

    Chunk {
        agent_id: AgentId,
        text: String,
    },

    Finished {
        agent_id: AgentId,
        elapsed: f64,
        ack: Option<oneshot::Sender<()>>,
    },

    Failed {
        agent_id: AgentId,
        error: String,
        ack: Option<oneshot::Sender<()>>,
    },

    Suspend {
        ack: oneshot::Sender<()>,
    },

    Resume {
        theme: AppTheme,
        history: VecDeque<ChatMessage>,
        ack: oneshot::Sender<()>,
    },

    Notice {
        role: MessageRole,
        message: String,
        ack: Option<oneshot::Sender<()>>,
    },

    Shutdown,
}

#[derive(Default)]
struct UiActorState {
    ui: UiState,
    terminal: Option<TerminalSession>,
}

impl UiActorState {
    fn start_terminal(&mut self) -> io::Result<()> {
        if self.terminal.is_none() {
            self.terminal = Some(TerminalSession::new()?);
        }

        Ok(())
    }

    fn restore_terminal(&mut self) -> io::Result<()> {
        if let Some(mut terminal) = self.terminal.take() {
            terminal.restore()?;
        }

        Ok(())
    }

    fn draw(&mut self, theme: &AppTheme) -> io::Result<()> {
        if let Some(terminal) = self.terminal.as_mut() {
            terminal.draw(&self.ui, theme)?;
        }

        Ok(())
    }

    fn start_agent(
        &mut self,
        agent_id: AgentId,
        model_id: String,
    ) -> io::Result<()> {
        self.start_terminal()?;

        self.ui.agents.insert(
            agent_id,
            AgentViewState::new(model_id),
        );

        self.ui.set_status("Generating");

        Ok(())
    }

    fn reasoning(
        &mut self,
        agent_id: AgentId,
        text: String,
    ) {
        let Some(agent) = self.ui.agents.get_mut(&agent_id) else {
            return;
        };

        agent.phase = AgentPhase::Reasoning;
        agent.reasoning.push_str(&text);
    }

    fn chunk(
        &mut self,
        agent_id: AgentId,
        text: String,
    ) {
        let Some(agent) = self.ui.agents.get_mut(&agent_id) else {
            return;
        };

        agent.phase = AgentPhase::Answering;
        agent.answer.push_str(&text);
    }

    fn finish(&mut self, agent_id: &str) {
        let Some(agent) = self.ui.agents.remove(agent_id) else {
            return;
        };

        self.ui.push_message(
            MessageRole::Assistant,
            agent.answer,
        );

        self.ui.set_status("Ready");
    }

    fn fail(
        &mut self,
        agent_id: &str,
        _error: String,
    ) {
        self.ui.agents.remove(agent_id);
        self.ui.set_status("Error");
    }

    async fn handle_key(
        &mut self,
        key: KeyEvent,
        action_tx: &mpsc::Sender<UiAction>,
    ) {
        let ctrl = key.modifiers.contains(
            crossterm::event::KeyModifiers::CONTROL
        );
    
        if ctrl && key.code == KeyCode::Char('c') {
            let _ = action_tx.send(UiAction::Interrupt).await;
            return;
        }
    
        if ctrl && key.code == KeyCode::Char('q') {
            let _ = action_tx.send(UiAction::Exit).await;
            return;
        }

        if self.ui.active_agents() > 0 {
            match key.code {
                KeyCode::PageUp => self.ui.scroll_up(10),
                KeyCode::PageDown => self.ui.scroll_down(10),
                KeyCode::End => self.ui.scroll_to_bottom(),
                _ => {}
            }
    
            return;
        }
    
        match self.ui.input.handle_edit_key(key) {
            InputEdit::Submit => {
                let input = self.ui.input.take();
                let trimmed = input.trim();
    
                if trimmed.is_empty() {
                    return;
                }
    
                let message = trimmed.to_string();
    
                self.ui.push_message(
                    MessageRole::User,
                    message.clone(),
                );
    
                self.ui.set_status("Processing");
    
                let _ = action_tx
                    .send(UiAction::Submit(message))
                    .await;
            }
    
            InputEdit::Cancel => {
                self.ui.input.clear();
            }
    
            InputEdit::Changed
            | InputEdit::Ignored => {}
        }
    
        match key.code {
            KeyCode::PageUp => self.ui.scroll_up(10),
            KeyCode::PageDown => self.ui.scroll_down(10),
            KeyCode::End => self.ui.scroll_to_bottom(),
            _ => {}
        }
    }

    async fn run(
        mut self,
        mut rx: mpsc::Receiver<UiEvent>,
        action_tx: mpsc::Sender<UiAction>,
        mut theme: AppTheme,
    ) -> anyhow::Result<()> {
        self.start_terminal()?;

        let mut ticker = interval(UI_FRAME_INTERVAL);

        ticker.set_missed_tick_behavior(
            MissedTickBehavior::Skip,
        );

        loop {
            tokio::select! {
                event = rx.recv() => {
                    let Some(event) = event else {
                        break;
                    };

                    match event {
                        UiEvent::Key(key) => {
                            self.handle_key(key, &action_tx).await;
                            self.draw(&theme)?;
                        }
                        
                        UiEvent::Resize { .. } => {
                            self.draw(&theme)?;
                        }
                        UiEvent::Started {
                            agent_id,
                            model_id,
                        } => {
                            self.start_agent(agent_id, model_id)?;
                            self.draw(&theme)?;
                        }

                        UiEvent::Reasoning {
                            agent_id,
                            text,
                        } => {
                            self.reasoning(agent_id, text);
                        }

                        UiEvent::Chunk {
                            agent_id,
                            text,
                        } => {
                            self.chunk(agent_id, text);
                        }

                        UiEvent::Finished {
                            agent_id,
                            elapsed,
                            ack,
                        } => {
                            self.finish(&agent_id);

                            self.ui.set_status(
                                format!("Finished in {elapsed:.2}s")
                            );

                            self.draw(&theme)?;

                            if let Some(ack) = ack {
                                let _ = ack.send(());
                            }
                        }

                        UiEvent::Failed {
                            agent_id,
                            error,
                            ack,
                        } => {
                            self.fail(&agent_id, error);

                            self.draw(&theme)?;

                            if let Some(ack) = ack {
                                let _ = ack.send(());
                            }
                        }

                        UiEvent::Suspend { ack } => {
                            self.restore_terminal()?;
                            let _ = ack.send(());
                        }

                        UiEvent::Resume {
                            theme: new_theme,
                            history,
                            ack,
                        } => {
                            theme = new_theme;
                        
                            self.ui.messages = history;
                            self.ui.scroll_to_bottom();
                        
                            self.start_terminal()?;
                            self.draw(&theme)?;
                        
                            let _ = ack.send(());
                        }
                        
                        UiEvent::Notice {
                            role,
                            message,
                            ack,
                        } => {
                            self.ui.push_notice(role, message);
                        
                            if self.terminal.is_some() {
                                self.draw(&theme)?;
                            }
                        
                            if let Some(ack) = ack {
                                let _ = ack.send(());
                            }
                        }

                        UiEvent::Shutdown => {
                            break;
                        }
                    }
                }

                _ = ticker.tick() => {
                    if self.terminal.is_some() {
                        self.draw(&theme)?;
                    }
                }
            }
        }

        self.restore_terminal()?;

        io::stdout().flush()?;

        Ok(())
    }
}

pub struct UiActor {
    tx: mpsc::Sender<UiEvent>,
    rx: Option<mpsc::Receiver<UiEvent>>,

    action_tx: mpsc::Sender<UiAction>,
    action_rx: Option<mpsc::Receiver<UiAction>>,
    
    task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl UiActor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(UI_CHANNEL_CAPACITY);
        let (action_tx, action_rx) = mpsc::channel(UI_ACTION_CAPACITY);

        Self {
            tx,
            rx: Some(rx),
            action_tx,
            action_rx: Some(action_rx),
            task: None,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<UiEvent> {
        self.tx.clone()
    }

    pub fn take_actions(
        &mut self,
    ) -> Option<mpsc::Receiver<UiAction>> {
        self.action_rx.take()
    }

    pub fn is_started(&self) -> bool {
        self.task.is_some()
    }
    
    pub fn start(
        &mut self,
        theme: AppTheme,
        history: VecDeque<ChatMessage>,
    ) {
        if self.task.is_some() {
            return;
        }
    
        let Some(rx) = self.rx.take() else {
            return;
        };
    
        let action_tx = self.action_tx.clone();
    
        self.task = Some(tokio::spawn(async move {
            let mut state = UiActorState::default();
    
            state.ui.messages = history;
    
            state.run(rx, action_tx, theme).await
        }));
    }

    pub async fn suspend(&self) -> anyhow::Result<()> {
        let (ack_tx, ack_rx) = oneshot::channel();

        self.tx
            .send(UiEvent::Suspend { ack: ack_tx })
            .await?;

        ack_rx.await?;

        Ok(())
    }

    pub async fn resume(
        &self,
        theme: AppTheme,
        history: VecDeque<ChatMessage>,
    ) -> anyhow::Result<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
    
        self.tx
            .send(UiEvent::Resume {
                theme,
                history,
                ack: ack_tx,
            })
            .await?;
    
        ack_rx.await?;
    
        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(task) = self.task.take() {
            let _ = self.tx.send(UiEvent::Shutdown).await;
            task.await??;
        }

        Ok(())
    }
}

impl Default for UiActor {
    fn default() -> Self {
        Self::new()
    }
}