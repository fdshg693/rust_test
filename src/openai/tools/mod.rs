//! Tools module aggregator.
//!
//! This reorganizes previous scattered tool builder files (`tool.rs`, `docs_tool.rs`, `tavily.rs`)
//! into a single `tools` namespace while keeping public re-exports stable.

mod core; // core definitions: ToolDefinition, ToolParameters, builders
mod sample_tools; // example constant-returning tool(s)
mod docs; // docs reading tool
mod tavily; // tavily search tool
mod rpg; // RPG game tools

pub use core::{
    ToolDefinition,
    ToolHandler,
    ToolParameters,
    ToolParametersBuilder,
};
pub use docs::build_read_doc_tool;
pub use tavily::{build_tavily_search_tool, tavily_search};
pub use sample_tools::{build_get_constants_tool, build_add_tool, build_number_guess_tool};
pub use rpg::{
    build_rpg_get_rules_tool,
    build_rpg_get_state_tool,
    build_rpg_list_actions_tool,
    build_rpg_issue_action_tool,
    build_rpg_tools,
};