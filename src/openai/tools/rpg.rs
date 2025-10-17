use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use serde_json::{json, Value};

use crate::rpg::{Game, Command, RpgRules};
use super::{ToolDefinition, ToolParametersBuilder};

// Provide a single shared game instance for tool-driven play.
// In a larger app, you might manage sessions keyed by an ID.
lazy_static! {
    static ref GAME: Mutex<Game> = Mutex::new(Game::new());
}

/// Tool: Return the RPG rules/config as JSON.
pub fn build_rpg_get_rules_tool() -> ToolDefinition {
    ToolDefinition::new(
        "rpg_get_rules",
        "Return the RPG rules/configuration so the model can understand game mechanics.",
        ToolParametersBuilder::new_object()
            .additional_properties(false)
            .build(),
        Arc::new(|_args| {
            let game = GAME.lock().unwrap();
            let rules: &RpgRules = &game.rules;
            Ok(serde_json::to_value(rules.clone())?)
        })
    )
}

/// Tool: Return the current game state snapshot.
pub fn build_rpg_get_state_tool() -> ToolDefinition {
    ToolDefinition::new(
        "rpg_get_state",
        "Return the current game state (player, enemy, turn, counters, rules subset).",
        ToolParametersBuilder::new_object()
            .additional_properties(false)
            .build(),
        Arc::new(|_args| {
            let game = GAME.lock().unwrap();
            let snap = game.snapshot();
            Ok(serde_json::to_value(snap)?)
        })
    )
}

/// Tool: List available player actions at this moment.
pub fn build_rpg_list_actions_tool() -> ToolDefinition {
    ToolDefinition::new(
        "rpg_list_actions",
        "List available actions the player can take now.",
        ToolParametersBuilder::new_object()
            .additional_properties(false)
            .build(),
        Arc::new(|_args| {
            let game = GAME.lock().unwrap();
            let mut actions = vec!["attack", "heal", "run", "quit"]; // static for now
            // Optional: filter by context, e.g., if zero potions, still allow heal but note it.
            if game.is_over() {
                actions = vec!["quit"];
            }
            Ok(json!({"actions": actions}))
        })
    )
}

/// Tool: Issue an action command. Returns updated snapshot and a simple log.
pub fn build_rpg_issue_action_tool() -> ToolDefinition {
    let params = ToolParametersBuilder::new_object()
        .add_string_enum(
            "action",
            Some("One of: attack, heal, run, quit"),
            &["attack", "heal", "run", "quit"],
        )
        .required("action")
        .additional_properties(false)
        .build();

    ToolDefinition::new(
        "rpg_issue_action",
        "Execute a player action and return the updated state.",
        params,
        Arc::new(|args: &Value| {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let cmd = match action {
                "attack" => Command::Attack,
                "heal" => Command::Heal,
                "run" => Command::Run,
                "quit" => Command::Quit,
                _ => return Ok(json!({"error": "invalid action"})),
            };

            let mut game = GAME.lock().unwrap();
            let cont = game.handle_command(cmd).unwrap_or(true);
            let snap = game.snapshot();
            Ok(json!({
                "continued": cont,
                "snapshot": serde_json::to_value(snap)?
            }))
        })
    )
}

/// Convenience: return all RPG-related tools as a single vector.
pub fn build_rpg_tools() -> Vec<ToolDefinition> {
    vec![
        build_rpg_get_rules_tool(),
        build_rpg_get_state_tool(),
        build_rpg_list_actions_tool(),
        build_rpg_issue_action_tool(),
    ]
}
