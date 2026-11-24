use anyhow::Result;
use colored::{Color as ColoredColor, Colorize};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::{Instant, Duration};
use std::io::{stdout, Write};
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
    dto::ChatSession,
    app::state::AppState,
    ui::TerminalUi,
    models::{
        capabilities::CapabilityKey
    }
};

pub async fn run_stream(
    state: &mut AppState,
    ui: &TerminalUi,
    prompt: &str,
) -> Result<()> {
    let start = Instant::now();

    let agent = state.agent.clone();
    let model_id = state.current_model.id.clone();
    let history = state.session.history.clone();

    let pb = ui.build_spinner(&model_id);

    stdout().flush()?;

    let mut stream = agent
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

    let mut printed_reasoning = false;
    let mut printed_text = false;

    while let Some(item) = stream.next().await {
        match item {
            // ---- thinking ----
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Reasoning { text, .. }),
            )) => {
                if can_think && show_thinking {
                    pb.finish_and_clear();
                    print!("{}", text.color(state.config.theme.dark_gray));
                    stdout().flush()?;
                    printed_reasoning = true;
                }
            }
            // ---- answer ----
            Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamItem::Event(StreamEvent::Text { text, .. }),
            )) => {
                pb.finish_and_clear();
                if printed_reasoning && !printed_text {
                    println!("\n");
                }
                print!("{text}");
                stdout().flush()?;
                printed_text = true;
            }
            // ---- CompletionCall ----
            Ok(MultiTurnStreamItem::CompletionCall(call)) => {
                let record = state.provider.parse_call_metrics(&model_id, &call.raw);
                if let Err(e) = state.record_call(record) {
                    eprintln!("\nWarning: failed to record call stats: {e}");
                }
            }
            // ---- final ----
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                pb.finish_and_clear();

                let strip = ChatSession::strip_reasoning(response.messages);
                state.session.history.push(Message::user(prompt));
                state.session.history.extend(strip);

                if let Err(e) = state.persist_session() {
                    eprintln!("\nWarning: failed to persist session: {e}");
                }

                let elapsed = start.elapsed();
                println!("\nFinished in {:.2}s", elapsed.as_secs_f64());
            }
            Err(err) => {
                pb.finish_and_clear();
                eprintln!("\n{model_id} error: {err}");
                break;
            }
            _ => {}
        }
    }
    Ok(())
}