use anyhow::Result;

use crate::{
    app::{
        AppContext,
        commands::execute_command,
    },
    ui::{
        mpsc_ui::UiAction,
    },
    agent::runtime::run_stream,
};

pub async fn chat_loop(ctx: &mut AppContext) -> Result<()> {
    let mut actions = ctx.ui
        .start_chat(&ctx.state.session.history,)?;
    
    loop {
        let Some(action) = actions.recv().await else {
            break;
        };

        match action {
            UiAction::Exit => {
                break;
            }

            UiAction::Submit(raw) => {
                let input = raw.trim();

                if input.is_empty() {
                    continue;
                }

                if input == "/exit" || input == "/q" {
                    break;
                }

                if input.starts_with('/') {
                    ctx.ui.suspend_chat().await?;
                
                    let command_result = execute_command(input, ctx).await;
                
                    let resume_result = ctx.ui
                        .resume_chat(&ctx.state.session.history)
                        .await;
                
                    command_result?;
                    resume_result?;
                
                    continue;
                }

                if let Err(error) = run_stream(ctx, input).await {
                    ctx.ui.error(
                        &format!("Agent error: {error:#}")
                    )?;
                }
            }
        }
    }

    ctx.ui.shutdown_output().await?;
    ctx.ui.goodbye()?;

    Ok(())
}