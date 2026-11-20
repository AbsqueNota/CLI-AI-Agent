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
use crate::dto::ChatSession;
use crate::{
    app::state::AppState,
    models::{
        capabilities::CapabilityKey
    }
};

pub async fn run_stream(state: &mut AppState,prompt: &str) -> Result<()> {
    let start = Instant::now();

    let agent = state.agent.clone();
    let model_id = state.current_model.id.clone();
    let history = state.session.history.clone();

    let spiner_part = "is thinking".color(state.config.theme.text_muted);
    let msg = format!("\n🤖: {} {spiner_part}", model_id.color(state.config.theme.text_light));

    let pb = build_spiner(state.config.theme.text_muted, msg);
    pb.enable_steady_tick(Duration::from_millis(150));

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


pub fn build_spiner(color: ColoredColor, msg:String) -> ProgressBar{
    let pb = ProgressBar::new_spinner();
    let spinner = [
        "".to_string(),
        ".".color(color).to_string(),
        "..".color(color).to_string(),
        "...".color(color).to_string(),
        "".to_string()
    ];
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&spinner.each_ref().map(|s| s.as_str()))
            .template("{msg}{spinner}")
            .unwrap(),
    );
    pb.set_message(msg);
    return pb;
}