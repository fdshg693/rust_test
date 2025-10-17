//! Example: Manually operate the RPG tools without any LLM
use rust_test::openai::tools::{
    build_rpg_get_rules_tool,
    build_rpg_get_state_tool,
    build_rpg_list_actions_tool,
    build_rpg_issue_action_tool,
};
use serde_json::json;

fn main() -> color_eyre::Result<()> {
    let _ = dotenvy::dotenv();
    color_eyre::install()?;

    let rules_tool = build_rpg_get_rules_tool();
    let state_tool = build_rpg_get_state_tool();
    let list_tool = build_rpg_list_actions_tool();
    let issue_tool = build_rpg_issue_action_tool();

    let rules = rules_tool.execute(&json!({}))?;
    println!("Rules: {}", rules);

    // Loop a few steps: attack until the game ends or after some iterations
    for _ in 0..5 {
        let state = state_tool.execute(&json!({}))?;
        println!("State: {}", state);
        let actions = list_tool.execute(&json!({}))?;
        println!("Actions: {}", actions);

        // naive policy: if heal available (potions>0 and HP not full), try heal once, otherwise attack
        // For simplicity here we just issue attack
        let result = issue_tool.execute(&json!({"action": "attack"}))?;
        println!("After action: {}", result);
    }
    Ok(())
}
