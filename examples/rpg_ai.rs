//! Example: Let the model play the RPG via tools (no TUI required)
use rust_test::config::Config;
use rust_test::openai::{
    build_rpg_tools,
    multi_step_tool_answer_blocking_with_logger,
    MultiStepLogEvent,
};

fn main() -> color_eyre::Result<()> {
    // Load .env if present and install pretty error reports
    let _ = dotenvy::dotenv();
    color_eyre::install()?;

    // Optional: simple logger to stdout
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let tools = build_rpg_tools();
    let prompt = r#"
You are going to play the Tiny CLI RPG using tools only.
- First, call rpg_get_rules to learn the mechanics.
- Then call rpg_get_state and rpg_list_actions to inspect.
- Repeatedly call rpg_issue_action until victory or defeat.
- When the game ends, provide a short summary as the final answer.
Reply with your final summary as plain text.
"#;

    let config = Config::default();
    let answer = multi_step_tool_answer_blocking_with_logger(
        prompt,
        &tools,
        &config,
        Some(10),
        |ev: &MultiStepLogEvent| {
            tracing::info!(target="example_rpg_ai", event=%ev, "multi_step_event");
        },
    )?;

    println!("=== FINAL ANSWER ===\n{}", answer.final_answer);
    Ok(())
}
