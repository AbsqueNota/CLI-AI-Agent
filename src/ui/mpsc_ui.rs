
use std::{
    io::{self, Write},
    time::Duration,
};

use colored::Colorize;

use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{
        interval,
        MissedTickBehavior,
    },
};

use crate::ui::{
    state::{
        AgentPhase,
        AgentViewState,
        MessageRole,
        UiState,
    },
    terminal::TerminalSession,
    theme::AppTheme,
};

pub type AgentId = String;

pub const UI_CHANNEL_CAPACITY: usize = 256;

const UI_FRAME_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Debug)]
pub enum UiEvent {
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
        ack: Option<tokio::sync::oneshot::Sender<()>>,
    },

    Failed {
        agent_id: AgentId,
        error: String,
        ack: Option<tokio::sync::oneshot::Sender<()>>,
    },

    Shutdown,
}

#[derive(Default)]
struct UiActorState {
    ui: UiState,
    terminal: Option<TerminalSession>,
    completed: Vec<(String, String)>,
    errors: Vec<String>,
}

impl UiActorState {
    fn start_agent(
        &mut self,
        agent_id: AgentId,
        model_id: String,
    ) -> io::Result<()> {
        if self.terminal.is_none() {
            self.terminal = Some(TerminalSession::new()?);
        }

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
            agent.answer.clone(),
        );
    
        self.completed.push((
            agent.model_id,
            agent.answer,
        ));
    }

    fn fail(
        &mut self,
        agent_id: &str,
        error: String,
    ) {
        self.ui.agents.remove(agent_id);
    
        self.ui.push_message(
            MessageRole::Error,
            error.clone(),
        );
    
        self.errors.push(error);
    }

    fn draw(&mut self, theme: &AppTheme) -> io::Result<()> {
        if let Some(terminal) = self.terminal.as_mut() {
            terminal.draw(&self.ui, theme)?;
        }

        Ok(())
    }

    fn restore_terminal(&mut self) -> io::Result<()> {
        if let Some(mut terminal) = self.terminal.take() {
            terminal.restore()?;
        }

        Ok(())
    }

    fn finish_generation(
        &mut self,
        theme: &AppTheme,
    ) -> io::Result<()> {
        self.draw(theme)?;
    
        if !self.ui.agents.is_empty() {
            return Ok(());
        }
    
        self.ui.set_status("Ready");
    
        self.restore_terminal()?;
    
        for (model_id, answer) in self.completed.drain(..) {
            println!(
                "\n🤖: {}\n{}",
                model_id.color(theme.palette.main),
                answer.color(theme.palette.default),
            );
        }
    
        for error in self.errors.drain(..) {
            eprintln!(
                "\n{}",
                error.color(AppTheme::error_color()),
            );
        }
    
        io::stdout().flush()?;
    
        Ok(())
    }

    async fn run(
        mut self,
        mut rx: mpsc::Receiver<UiEvent>,
        theme: AppTheme,
    ) -> anyhow::Result<()> {
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
                        UiEvent::Started {
                            agent_id,
                            model_id,
                        } => {
                            self.start_agent(
                                agent_id,
                                model_id,
                            )?;

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
                            self.finish_generation(&theme)?;

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
                            self.finish_generation(&theme)?;                        
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
                    self.draw(&theme)?;
                }
            }
        }

        self.restore_terminal()?;

        Ok(())
    }
}

pub struct UiActor {
    tx: mpsc::Sender<UiEvent>,
    rx: Option<mpsc::Receiver<UiEvent>>,
    task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl UiActor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(UI_CHANNEL_CAPACITY);

        Self {
            tx,
            rx: Some(rx),
            task: None,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<UiEvent> {
        self.tx.clone()
    }

    pub fn start(&mut self, theme: AppTheme) {
        if self.task.is_some() {
            return;
        }

        let Some(rx) = self.rx.take() else {
            return;
        };

        self.task = Some(tokio::spawn(async move {
            UiActorState::default()
                .run(rx, theme)
                .await
        }));
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
