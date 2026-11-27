use std::time::Instant;
use uuid::Uuid;
use anyhow::{Context, Result};
use tokio_stream::StreamExt;

use rig::{
    agent::MultiTurnStreamItem,
    message::Message,
    streaming::{
        Item as StreamItem,
        StreamEvent,
    },
};

use crate::{
    app::AppContext,
    dto::ChatSession,
    models::capabilities::CapabilityKey,
    ui::{
        mpsc_ui::UiEvent,
    },
};

pub async fn run_stream(
    ctx: &mut AppContext,
    prompt: &str,
) -> Result<()> {
    let AppContext { state, ui } = ctx;

    let start = Instant::now();

    let agent_id = Uuid::now_v7().to_string();
    let model_id = state.current_model.id.clone();

    let ui_tx = ui.event_sender();

    ui_tx
        .send(UiEvent::Started {
            agent_id: agent_id.clone(),
            model_id: model_id.clone(),
        })
        .await
        .context("Failed to send UI Started event")?;

    let can_think = state
        .current_model
        .capabilities
        .supports(CapabilityKey::Thinking);

    let show_thinking = state
        .current_model
        .settings
        .thinking
        .as_ref()
        .is_some_and(|settings| settings.is_visible);

    let mut stream = state.agent
        .prompt(prompt)
        .history(state.session.history.clone())
        .stream();

    let mut completed = false;
    let mut stream_error = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Reasoning { text, .. }),
            )) => {
                if can_think && show_thinking {
                    ui_tx
                        .send(UiEvent::Reasoning {
                            agent_id: agent_id.clone(),
                            text,
                        })
                        .await
                        .context("Failed to send UI Reasoning event")?;
                }
            }

            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Text { text, .. }),
            )) => {
                ui_tx
                    .send(UiEvent::Chunk {
                        agent_id: agent_id.clone(),
                        text,
                    })
                    .await
                    .context("Failed to send UI Chunk event")?;
            }

            Ok(MultiTurnStreamItem::CompletionCall(call)) => {
                let record = state
                    .provider
                    .parse_call_metrics(&model_id, &call.raw);

                if let Err(error) = state.record_call(record).await {
                    ui.warning(
                        &format!("Failed to record call stats: {error}"),
                    )?;
                }
            }

            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                let messages = ChatSession::strip_reasoning(
                    response.messages,
                );

                state.session.history.push(Message::user(prompt));
                state.session.history.extend(messages);

                state.persist_session();

                completed = true;
                break;
            }

            Err(error) => {
                stream_error = Some(format!("{model_id}: {error}"));
                break;
            }

            _ => {}
        }
    }

    drop(stream);

    if let Some(error) = stream_error {
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    
        ui_tx
            .send(UiEvent::Failed {
                agent_id,
                error: error.to_string(),
                ack: Some(ack_tx),
            })
            .await
            .context("Failed to send UI Failed event")?;
    
        ack_rx
            .await
            .context("UI actor stopped before processing Failed event")?;
    
        return Err(anyhow::anyhow!(error));
    }

    if !completed {
        let error = anyhow::anyhow!(
            "{model_id}: stream ended without FinalResponse"
        );
    
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    
        ui_tx
            .send(UiEvent::Failed {
                agent_id,
                error: error.to_string(),
                ack: Some(ack_tx),
            })
            .await
            .context("Failed to send UI Failed event")?;
    
        ack_rx
            .await
            .context("UI actor stopped before processing Failed event")?;
    
        return Err(error);
    }

    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    ui_tx
        .send(UiEvent::Finished {
            agent_id,
            elapsed: start.elapsed().as_secs_f64(),
            ack: Some(ack_tx),
        })
        .await
        .context("Failed to send UI Finished event")?;

    ack_rx
        .await
        .context("UI actor stopped before processing Finished event")?;
    
    Ok(())
}
