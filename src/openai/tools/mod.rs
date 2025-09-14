//! Tools module aggregator.
//!
//! This reorganizes previous scattered tool builder files (`tool.rs`, `docs_tool.rs`, `tavily.rs`)
//! into a single `tools` namespace while keeping public re-exports stable.

mod core; // core definitions: ToolDefinition, ToolParameters, builders
mod constants; // example constant-returning tool(s)
mod docs; // docs reading tool
mod tavily; // tavily search tool

pub use core::{
    ToolDefinition,
    ToolHandler,
    ToolParameters,
    ToolParametersBuilder,
};
pub use docs::build_read_doc_tool;
pub use tavily::{build_tavily_search_tool, tavily_search};
pub use constants::build_get_constants_tool;