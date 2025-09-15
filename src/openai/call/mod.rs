// Submodule for tool-calling features: types, proposer, resolver, and multi-step orchestration.

pub mod types;
pub mod proposer;
pub mod resolver;
pub mod multi_step;

// Re-export commonly used items to keep external API stable via openai::call::* if needed.
pub use types::{ToolCallDecision, ToolResolution, MultiStepAnswer, MultiStepLogEvent};
pub use proposer::{propose_tool_call, propose_tool_call_blocking};
pub use resolver::resolve_and_execute_tool_call;
pub use multi_step::{
    multi_step_tool_answer,
    multi_step_tool_answer_blocking,
    multi_step_tool_answer_with_logger,
    multi_step_tool_answer_blocking_with_logger,
};
