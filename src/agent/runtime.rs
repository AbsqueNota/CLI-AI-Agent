use anyhow::Result;
use std::time::Instant;
use tokio_stream::StreamExt;
use rig::{
    message::Message,
    agent::MultiTurnStreamItem,
    streaming::{
        Item as StreamItem,
        StreamEvent,
    }
};
use crate::{
    ui::TerminalUi,
    app::AppContext,
    dto::ChatSession,
    models::{
        capabilities::CapabilityKey
    }
};


pub async fn run_stream(
    ctx: &mut AppContext,
    prompt: &str,
) -> Result<()> {
    let AppContext { state, ui } = ctx;
    
    let start = Instant::now();

    let agent = state.agent.clone();
    let model_id = state.current_model.id.clone();
    let history = state.session.history.clone();

    let pb = ui.build_spinner(&model_id)?; 

    let mut stream = agent.inner
        .prompt(prompt)
        .history(history)
        .stream();

    let can_think = state.current_model.capabilities.supports(CapabilityKey::Thinking);
    let show_thinking = state
        .current_model
        .settings
        .thinking
        .as_ref()
        .is_some_and(|t| t.is_visible);

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Reasoning { text, .. }),
            )) => {
                if can_think && show_thinking {
                    pb.finish_and_clear();
                    ui.write_reasoning(&text)?;
                }
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Text { text, .. }),
            )) => {
                pb.finish_and_clear();
                ui.write_text(&text)?;         
            }
            Ok(MultiTurnStreamItem::CompletionCall(call)) => {
                let record = state.provider.parse_call_metrics(&model_id, &call.raw);
                if let Err(error) = state.record_call(record).await {
                    TerminalUi::warning(
                        &format!("Failed to record call stats: {error}")
                    );
                }
            }
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                pb.finish_and_clear();

                let strip = ChatSession::strip_reasoning(response.messages);
                state.session.history.push(Message::user(prompt));
                state.session.history.extend(strip);

                state.persist_session();
            
                ui.stream_finished(start.elapsed().as_secs_f64());
            }
            Err(err) => {
                pb.finish_and_clear();
                TerminalUi::stream_error(&model_id, &err);
                break;
            }
            _ => {}
        }
    }
    Ok(())
}