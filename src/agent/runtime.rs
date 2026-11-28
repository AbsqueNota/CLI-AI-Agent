use std::time::Instant;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use uuid::Uuid;

use rig::{
    agent::MultiTurnStreamItem,
    message::Message,
    streaming::{
        Item as StreamItem,
        StreamEvent,
    },
};

use crate::{
    agent::AgentFactory,
    app::events::AppEvent,
    dto::ChatSession,
    models::Model,
    provider::LlmProvider,
};

pub fn spawn_agent(
    agent_id: String,
    provider: std::sync::Arc<dyn LlmProvider>,
    model: Model,
    history: Vec<Message>,
    prompt: String,
    tx: mpsc::Sender<AppEvent>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let model_id = model.id.clone();
        let start = Instant::now();

        let _ = tx.send(AppEvent::AgentStarted {
            agent_id: agent_id.clone(),
            model_id: model_id.clone(),
        }).await;

        let agent = match AgentFactory::build(&*provider, &model) {
            Ok(agent) => agent,
            Err(error) => {
                let _ = tx.send(AppEvent::AgentFailed {
                    agent_id,
                    error: format!("{model_id}: {error:#}"),
                }).await;
                return;
            }
        };

        let mut stream = agent
            .prompt(&prompt)
            .history(history)
            .stream();

        let mut completed = false;

        while let Some(item) = stream.next().await {
            match item {
                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamItem::Event(StreamEvent::Reasoning { text, .. }),
                )) => {
                    let _ = tx.send(AppEvent::AgentReasoning {
                        agent_id: agent_id.clone(),
                        text,
                    }).await;
                }

                Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamItem::Event(StreamEvent::Text { text, .. }),
                )) => {
                    let _ = tx.send(AppEvent::AgentChunk {
                        agent_id: agent_id.clone(),
                        text,
                    }).await;
                }

                Ok(MultiTurnStreamItem::CompletionCall(call)) => {
                    let record = provider.parse_call_metrics(
                        &model_id,
                        &call.raw,
                    );

                    let _ = tx.send(AppEvent::AgentCall {
                        agent_id: agent_id.clone(),
                        record,
                    }).await;
                }

                Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                    let messages = ChatSession::strip_reasoning(
                        response.messages,
                    );

                    let _ = tx.send(AppEvent::AgentFinished {
                        agent_id: agent_id.clone(),
                        prompt,
                        messages,
                        elapsed: start.elapsed().as_secs_f64(),
                    }).await;

                    completed = true;
                    break;
                }

                Err(error) => {
                    let _ = tx.send(AppEvent::AgentFailed {
                        agent_id: agent_id.clone(),
                        error: format!("{model_id}: {error}"),
                    }).await;
                    return;
                }

                _ => {}
            }
        }

        if !completed {
            let _ = tx.send(AppEvent::AgentFailed {
                agent_id,
                error: format!(
                    "{model_id}: stream ended without FinalResponse"
                ),
            }).await;
        }
    })
}