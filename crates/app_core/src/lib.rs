//! app_core
//! 
//! OpenAI API連携、RPGゲームロジック、SQLite操作などの共通ロジックを提供するコアクレート。
//! TUI/Web両方で利用可能。

pub mod config;
pub mod openai;
pub mod rpg;
pub mod sqlite;
pub mod services;

// 主要な型を再エクスポート
pub use config::OpenAIConfig;
pub use services::{ChatService, RpgService};
