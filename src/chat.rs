use anyhow::Result;
use crate::{
    app::{
        AppContext,
        commands::execute_command,
    },
    agent::runtime::run_stream,
};

pub async fn chat_loop(ctx: &mut AppContext) -> Result<()> {
    ctx.ui.welcome();
   
    loop {
        ctx.ui.model_prompt(&ctx.state.current_model.id);

        let raw = ctx.ui.input("You",false)?;
        let input = raw.trim();

        ctx.ui.echo_input("You", input)?;

        if input.is_empty() {
            continue;
        }

        if input == "/exit" || input == "/q" {
            ctx.ui.goodbye();
            break;
        }

        if input.starts_with('/') {
            execute_command(input, ctx).await?;
            continue;
        }
        
        if let Err(error) = run_stream(ctx, input).await {
            ctx.ui.error(&format!("Agent error: {error}"));
        }
    }

    Ok(())
}
