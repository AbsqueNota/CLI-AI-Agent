use std::time::Instant;

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
        TerminalUi,
        mpsc_ui::UiEvent,
    },
};

pub async fn run_stream(
    ctx: &mut AppContext,
    prompt: &str,
) -> Result<()> {
    let AppContext { state, ui } = ctx;

    let start = Instant::now();

    let agent = state.agent.clone();
    let model_id = state.current_model.id.clone();

    let ui_tx = ui.event_sender();

    ui_tx
        .send(UiEvent::Started {
            agent_id: agent.id.clone(),
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

    let mut stream = agent
        .inner
        .prompt(prompt)
        .history(state.session.history.clone())
        .stream();

    let mut completed = false;

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Reasoning { text, .. }),
            )) => {
                if can_think && show_thinking {
                    ui_tx
                        .send(UiEvent::Reasoning {
                            agent_id: agent.id.clone(),
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
                        agent_id: agent.id.clone(),
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
                    TerminalUi::warning(
                        &format!("Failed to record call stats: {error}"),
                    );
                }
            }

            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                let messages = ChatSession::strip_reasoning(
                    response.messages,
                );

                state.session.history.push(Message::user(prompt));
                state.session.history.extend(messages);

                state.persist_session();

                ui_tx
                    .send(UiEvent::Finished {
                        agent_id: agent.id.clone(),
                        elapsed: start.elapsed().as_secs_f64(),
                    })
                    .await
                    .context("Failed to send UI Finished event")?;

                completed = true;
                break;
            }

            Err(error) => {
                ui_tx
                    .send(UiEvent::Failed {
                        agent_id: agent.id.clone(),
                        error: format!("{model_id}: {error}"),
                    })
                    .await
                    .context("Failed to send UI Failed event")?;

                break;
            }

            _ => {}
        }
    }

    drop(stream);

    if !completed {
        ui_tx
            .send(UiEvent::Failed {
                agent_id: agent.id,
                error: format!(
                    "{model_id}: stream ended without FinalResponse"
                ),
            })
            .await
            .context("Failed to send UI Failed event")?;
    }

    Ok(())
}
