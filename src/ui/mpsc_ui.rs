
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
        ack: Option<tokio::sync::oneshot::Sender<()>>,
    },
    Failed {
        agent_id: AgentId,
        error: String,
    },
    Shutdown,
}

#[derive(Default)]
struct AgentOutputState {
    model_id: String,
    has_reasoning: bool,
    has_answer: bool,
}

#[derive(Default)]
struct UiState {
    agents: HashMap<AgentId, AgentOutputState>,
}

impl UiState {
    async fn run(
        &mut self,
        mut rx: mpsc::Receiver<UiEvent>,
        theme: AppTheme,
    ) {
        let mut active_agents = HashSet::<AgentId>::new();
        let mut spinner: Option<ProgressBar> = None;
        let mut content_started = false;

        while let Some(event) = rx.recv().await {
            match event {
                UiEvent::Started {
                    agent_id,
                    model_id,
                } => {
                    if active_agents.is_empty() {
                        content_started = false;

                        spinner = Some(build_spinner(
                            &theme,
                            &model_id,
                        ));
                    }

                    active_agents.insert(agent_id.clone());

                    self.agents.insert(
                        agent_id,
                        AgentOutputState {
                            model_id,
                            ..Default::default()
                        },
                    );
                }

                UiEvent::Reasoning {
                    agent_id,
                    text,
                } => {
                    if !content_started {
                        if let Some(pb) = spinner.take() {
                            pb.finish_and_clear();
                        }

                        content_started = true;
                    }

                    let (model_id, first_reasoning) = {
                        let state = self
                            .agents
                            .entry(agent_id.clone())
                            .or_default();

                        let first = !state.has_reasoning;
                        state.has_reasoning = true;
                        (state.model_id.clone(), first)
                    };

                    if first_reasoning {
                        print!(
                            "{}{}{} {}{}\n",
                            "[".color(theme.palette.muted),
                            model_id.color(theme.palette.main).bold(),
                            "]".color(theme.palette.muted),
                            "think".color(theme.palette.muted),
                            ":".color(theme.palette.muted),
                        );
                    }

                    print!(
                        "{}",
                        text.color(theme.palette.muted)
                    );

                    let _ = io::stdout().flush();
                }

                UiEvent::Chunk {
                    agent_id,
                    text,
                } => {
                    if !content_started {
                        if let Some(pb) = spinner.take() {
                            pb.finish_and_clear();
                        }

                        content_started = true;
                    }

                    let (model_id, first_answer, needs_new_line) = {
                        let state = self
                            .agents
                            .entry(agent_id.clone())
                            .or_default();

                        let first = !state.has_answer;
                        let needs_new_line =
                            state.has_reasoning && first;

                        state.has_answer = true;

                        (
                            state.model_id.clone(),
                            first,
                            needs_new_line,
                        )
                    };

                    if needs_new_line {
                        println!();
                    }

                    if first_answer {
                        print!(
                            "{}{}{} {}{}\n",
                            "[".color(theme.palette.muted),
                            model_id.color(theme.palette.main).bold(),
                            "]".color(theme.palette.muted),
                            "answer".color(theme.palette.muted),
                            ":".color(theme.palette.muted),
                        );
                    }

                    print!(
                        "{}",
                        text.color(theme.palette.default)
                    );

                    let _ = io::stdout().flush();
                }

                UiEvent::Finished {
                    agent_id,
                    elapsed,
                    ack,
                } => {
                    let model_id = self.agents
                        .get(&agent_id)
                        .map(|state| state.model_id.as_str())
                        .unwrap_or_else(|| agent_id.as_str());

                    println!(
                        "\n{} {}{}{} {} {}\n",
                        "Finished".color(theme.palette.info),
                        "[".color(theme.palette.info),
                        model_id.color(theme.palette.main).bold(),
                        "]".color(theme.palette.info),
                        "in".color(theme.palette.info),
                        format!("{elapsed:.2}s").color(theme.palette.info),
                    );

                    active_agents.remove(&agent_id);
                    self.agents.remove(&agent_id);

                    if let Some(ack) = ack {
                        let _ = ack.send(());
                    }
                }

                UiEvent::Failed {
                    agent_id,
                    error,
                } => {
                    let model_id = self.agents
                        .get(&agent_id)
                        .map(|state| state.model_id.as_str())
                        .unwrap_or_else(|| agent_id.as_str());

                    eprintln!(
                        "\n{}",
                        format!(
                            "[{}] {}",
                            model_id,
                            error,
                        )
                        .color(AppTheme::error_color())
                    );

                    active_agents.remove(&agent_id);
                    self.agents.remove(&agent_id);
                }

                UiEvent::Shutdown => {
                    break;
                }
            }

            if active_agents.is_empty() {
                if let Some(pb) = spinner.take() {
                    pb.finish_and_clear();
                }

                content_started = false;
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

fn build_spinner(
    theme: &AppTheme,
    model_id: &str,
) -> ProgressBar {
    let message = format!(
        "\n🤖: {} {}",
        model_id.color(theme.palette.main),
        "is thinking".color(theme.palette.muted),
    );

    let indicator_color = theme.palette.muted;

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
