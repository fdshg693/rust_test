//! OpenAI連携のモジュール（ファイル分割版）

pub mod worker;
pub mod simple;
pub mod call; // tool-calling (types, proposer, resolver, multi-step)
pub mod tools; // consolidated tools (core, docs, tavily, constants)
pub mod history; // conversation history helper

// 代表的な公開APIを再エクスポート
pub use worker::start_openai_worker;
pub use simple::{
	get_ai_answer_once,
	get_ai_answer_once_blocking,	
};
pub use call::{
	ToolCallDecision,
	ToolResolution,
	MultiStepAnswer,
	MultiStepLogEvent,
	propose_tool_call,
	propose_tool_call_blocking,
	resolve_and_execute_tool_call,
	multi_step_tool_answer,
	multi_step_tool_answer_blocking,
	multi_step_tool_answer_with_logger,
	multi_step_tool_answer_blocking_with_logger,
	request_chat_completion
};
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
	build_number_guess_tool,
	build_rpg_get_rules_tool,
	build_rpg_get_state_tool,
	build_rpg_list_actions_tool,
	build_rpg_issue_action_tool,
	build_rpg_tools,
};
