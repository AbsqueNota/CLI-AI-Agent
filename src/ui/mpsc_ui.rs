use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    time::Duration,
};

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{
    sync::mpsc,
    task::JoinHandle,
};

use crate::ui::theme::AppTheme;

pub type AgentId = String;

pub const UI_CHANNEL_CAPACITY: usize = 256;

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
    },
    Failed {
        agent_id: AgentId,
        error: String,
    },
    Shutdown,
}

#[derive(Debug, Default)]
pub struct AgentBuffer {
    pub model_id: String,
    pub reasoning: String,
    pub output: String,
    pub finished: bool,
    pub elapsed: Option<f64>,
    pub error: Option<String>,

    is_printed_reasoning: bool,
    is_printed_text: bool,
}

#[derive(Default)]
pub struct UiState {
    pub agents: HashMap<AgentId, AgentBuffer>,
}

impl UiState {
    pub fn handle(&mut self, event: &UiEvent) {
        match event {
            UiEvent::Started {
                agent_id,
                model_id,
            } => {
                self.agents.insert(
                    agent_id.clone(),
                    AgentBuffer {
                        model_id: model_id.clone(),
                        ..Default::default()
                    },
                );
            }

            UiEvent::Reasoning {
                agent_id,
                text,
            } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.reasoning.push_str(text);
                    agent.is_printed_reasoning = true;
                }
            }

            UiEvent::Chunk {
                agent_id,
                text,
            } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.output.push_str(text);
                    agent.is_printed_text = true;
                }
            }

            UiEvent::Finished {
                agent_id,
                elapsed,
            } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.finished = true;
                    agent.elapsed = Some(*elapsed);
                }
            }

            UiEvent::Failed {
                agent_id,
                error,
            } => {
                if let Some(agent) = self.agents.get_mut(agent_id) {
                    agent.error = Some(error.clone());
                    agent.finished = true;
                }
            }

            UiEvent::Shutdown => {}
        }
    }

    async fn run(
        &mut self,
        mut rx: mpsc::Receiver<UiEvent>,
        theme: AppTheme,
    ) {
        let mut active_agents = HashSet::<AgentId>::new();
        let mut spinner: Option<ProgressBar> = None;
        let mut output_started = false;

        while let Some(event) = rx.recv().await {
            match &event {
                UiEvent::Started {
                    agent_id,
                    model_id,
                } => {
                    if active_agents.is_empty() {
                        output_started = false;

                        spinner = Some(build_spinner(
                            &theme,
                            model_id,
                        ));
                    }

                    active_agents.insert(agent_id.clone());
                }

                UiEvent::Reasoning {
                    agent_id,
                    text,
                } => {
                    if !output_started {
                        if let Some(pb) = spinner.take() {
                            pb.finish_and_clear();
                        }

                        output_started = true;
                    }

                    if let Some(agent) = self.agents.get(agent_id) {
                        let id = short_agent_id(agent_id);

                        print!(
                            "[{id}:thinking] {}",
                            text.color(theme.palette.model.thinking)
                        );

                        io::stdout().flush().ok();

                        let _ = agent;
                    }
                }

                UiEvent::Chunk {
                    agent_id,
                    text,
                } => {
                    if !output_started {
                        if let Some(pb) = spinner.take() {
                            pb.finish_and_clear();
                        }
                
                        output_started = true;
                    }
                
                    let needs_new_line = self
                        .agents
                        .get(agent_id)
                        .is_some_and(|agent| {
                            agent.is_printed_reasoning
                                && !agent.is_printed_text
                        });
                
                    if needs_new_line {
                        println!("\n");
                    }
                
                    let id = short_agent_id(agent_id);
                
                    print!(
                        "[{id}:answer] {}",
                        text.color(theme.palette.model.answer)
                    );
                
                    io::stdout().flush().ok();
                }

                UiEvent::Finished {
                    agent_id,
                    elapsed,
                } => {
                    if let Some(pb) = spinner.take() {
                        if active_agents.len() <= 1 {
                            pb.finish_and_clear();
                        } else {
                            // Другие агенты ещё работают.
                            spinner = Some(pb);
                        }
                    }

                    println!(
                        "\nFinished [{}] in {:.2}s",
                        short_agent_id(agent_id),
                        elapsed,
                    );

                    active_agents.remove(agent_id);
                }

                UiEvent::Failed {
                    agent_id,
                    error,
                } => {
                    if let Some(pb) = spinner.take() {
                        if active_agents.len() <= 1 {
                            pb.finish_and_clear();
                        } else {
                            spinner = Some(pb);
                        }
                    }

                    eprintln!(
                        "\n{}",
                        format!(
                            "[{}] {}",
                            short_agent_id(agent_id),
                            error,
                        )
                        .color(AppTheme::error_color())
                    );

                    active_agents.remove(agent_id);
                }

                UiEvent::Shutdown => {
                    if let Some(pb) = spinner.take() {
                        pb.finish_and_clear();
                    }

                    break;
                }
            }

            self.handle(&event);

            if active_agents.is_empty() {
                if let Some(pb) = spinner.take() {
                    pb.finish_and_clear();
                }

                output_started = false;
            }
        }

        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }
    }
}

pub struct UiActor {
    tx: mpsc::Sender<UiEvent>,
    rx: Option<mpsc::Receiver<UiEvent>>,
    task: Option<JoinHandle<()>>,
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
            UiState::default().run(rx, theme).await;
        }));
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(task) = self.task.take() {
            // Если receiver уже завершился, отправлять Shutdown
            // больше не требуется.
            let _ = self.tx.send(UiEvent::Shutdown).await;

            task.await?;
        }

        Ok(())
    }
}

impl Default for UiActor {
    fn default() -> Self {
        Self::new()
    }
}

fn short_agent_id(agent_id: &str) -> &str {
    &agent_id[..agent_id.len().min(8)]
}

fn build_spinner(
    theme: &AppTheme,
    model_id: &str,
) -> ProgressBar {
    let message = format!(
        "\n🤖: {} {}",
        model_id.color(theme.palette.model.answer),
        "is thinking".color(theme.palette.spinner.message),
    );

    let indicator_color = theme.palette.spinner.indicator;

    let ticks = [
        "".to_string(),
        ".".color(indicator_color).to_string(),
        "..".color(indicator_color).to_string(),
        "...".color(indicator_color).to_string(),
        "".to_string(),
    ];

    let pb = ProgressBar::new_spinner();

    let style = ProgressStyle::default_spinner()
        .tick_strings(
            &ticks.each_ref().map(|s| s.as_str()),
        )
        .template("{msg}{spinner}")
        .expect("Invalid spinner template");

    pb.set_style(style);
    pb.set_message(message);
    pb.enable_steady_tick(Duration::from_millis(150));

    pb
}