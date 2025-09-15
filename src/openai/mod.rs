//! OpenAI連携のモジュール（ファイル分割版）

pub mod worker;
pub mod simple;
pub mod call_tool;
pub mod tools; // consolidated tools (core, docs, tavily, constants)
pub mod history; // conversation history helper

// 代表的な公開APIを再エクスポート
pub use worker::start_openai_worker;
pub use simple::{
	get_ai_answer_once,
	get_ai_answer_once_blocking,	
};
pub use call_tool::{
	propose_tool_call,
	propose_tool_call_blocking,
	// Convenience wrapper (Vec-based) will also be available once defined
	// propose_tool_call_with_history_vec (re-exported below)
	ToolCallDecision,
	ToolResolution,
	resolve_and_execute_tool_call,
	multi_step_tool_answer,
	multi_step_tool_answer_blocking,
	multi_step_tool_answer_with_logger,
	multi_step_tool_answer_blocking_with_logger,
	MultiStepAnswer,
    MultiStepLogEvent,
};
pub use call_tool::propose_tool_call_with_history_vec;
pub use history::ConversationHistory;
pub use tools::{
	ToolDefinition,
	ToolHandler,
	ToolParameters,
	ToolParametersBuilder,
	build_get_constants_tool,
	build_add_tool,
	build_read_doc_tool,
	build_tavily_search_tool,
	tavily_search,
};
