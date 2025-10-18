# Rust TUI Application - Multi-Mode System

This is a Rust TUI application using ratatui with a multi-mode system (Menu, OpenAI Chat, RPG Game).

## Project Structure

```
crates/
├── app_cli/             # CLI/TUI application
│   ├── src/
│   │   ├── main.rs      # Application entry point
│   │   ├── lib.rs       # Main loop and mode-driven architecture
│   │   ├── modes/       # Mode system (Menu, OpenAI Chat, RPG)
│   │   └── ui/          # UI components
│   └── Cargo.toml
├── app_core/            # Core business logic (shared)
│   ├── src/
│   │   ├── config.rs    # Configuration and constants
│   │   ├── openai/      # OpenAI API integration with tool calling
│   │   ├── rpg/         # RPG game logic
│   │   ├── services/    # Business logic services
│   │   └── sqlite/      # SQLite database utilities
│   └── Cargo.toml
└── app_web/             # Web application (Axum)
    ├── src/
    │   ├── main.rs      # Web server entry point
    │   ├── handlers/    # API handlers
    │   └── models/      # Request/response models
    ├── static/          # Static assets
    ├── templates/       # HTML templates
    └── Cargo.toml
```

## Features

### 1. Multi-Mode System (Phase 4 ✅)
- **Menu Mode**: Startup screen to select OpenAI Chat, RPG Game, or Exit
- **OpenAI Chat Mode**: Interactive chat with OpenAI's API
- **RPG Game Mode**: Text-based RPG adventure game
- Easy mode switching with Esc to return to menu

### 2. OpenAI Integration (✅ Web/CLI両対応)
- Background worker thread for API calls (CLI)
- Async service layer for web handlers (Web)
- **Tool calling support with Send boundary** - Web/CLI両方で利用可能
  - Number guessing tool
  - Get constants tool
  - Add tool
  - RPG tools
  - TAVILY search tool
- Multi-step tool resolution with logging
- Streaming-like responses with real-time updates (CLI)

### 3. RPG Game
- Character stats (HP, MP, Attack Power)
- Combat system with A (Attack), H (Heal), R (Rest), Q (Quit)
- Procedural enemy generation
- Win/lose conditions

### 4. SQLite Storage
- Simple key-value storage with file metadata
- API: `upsert_text`, `read_text`, `list_files`, etc.
- Supports both text and binary data

## Architecture

### Mode Trait
```rust
pub trait Mode {
    fn update(&mut self);  // Non-blocking async result checking
    fn render(&self, f: &mut Frame);  // Immutable rendering
    fn handle_key(&mut self, key: KeyEvent) -> Result<Option<AppMode>>;
}
```

### Mode Lifecycle
1. `MenuMode` starts at launch
2. User selects a mode via Up/Down/Enter
3. Each mode has its own state, rendering, and event handling
4. Esc returns to menu
5. Exit from menu terminates the application

## Usage

### Menu Mode (Startup)
```
Use Up/Down arrow keys to select a mode
Press Enter to select
Press Esc to exit
```

### OpenAI Chat Mode
```
Type your message and press Enter to submit
Press Backspace to delete characters
View AI responses in real-time
Press Esc to return to menu
```

### RPG Game Mode
```
Press A - Attack
Press H - Heal (costs MP)
Press R - Rest (restore HP and MP)
Press Q - Quit game
Press Esc - Return to menu
```

## Running the Application

### CLI/TUI Application

```bash
# Development mode
cargo run -p app_cli

# Release mode (optimized)
cargo run -p app_cli --release

# With logging
RUST_LOG=app=debug,openai=debug cargo run -p app_cli
```

### Web Application

```bash
# Development mode
cargo run -p app_web

# Then open browser at:
# - Home: http://localhost:3000
# - Chat: http://localhost:3000/chat
# - RPG: http://localhost:3000/rpg

# With logging
RUST_LOG=web=debug,chat_service=debug cargo run -p app_web
```

### General Commands

```bash
# Check code
cargo check

# Run tests
cargo test

# Run specific package tests
cargo test -p app_core
```

## Environment Variables

### Required
- `OPENAI_API_KEY`: Your OpenAI API key

### Optional
- `TAVILY_API_KEY`: For web search functionality
- `RUST_LOG`: Control logging level (e.g., `app=debug,openai=debug`)

## Configuration

Edit `src/config.rs` to customize:
- `MODEL`: Default OpenAI model (currently `gpt-4o-mini`)
- `MAX_TOKENS`: Max tokens per response
- Game parameters and OpenAI settings

## Development Guide

See `.github/copilot-instructions.md` for:
- Architecture patterns
- Adding new modes
- Extending OpenAI tools
- Best practices for this codebase

## TAVILY Search Tool

OpenAI function calling integration with web search:
- Tool name: `TAVILY_search`
- Arguments: `query` (required), `max_results` (optional, 1-10)
- Requires: `TAVILY_API_KEY` environment variable

## Technical Details

### OpenAI Tool Calling with Send Boundary

Web版とCLI版の両方でOpenAI tool calling機能を使用するため、`multi_step_tool_answer_with_logger`関数のクロージャに`Send`トレイト境界を追加しています。

```rust
pub async fn multi_step_tool_answer_with_logger<F>(
    original_user_prompt: &str,
    tools: &[ToolDefinition],
    config: &OpenAIConfig,
    max_loops: Option<usize>,
    logger: F,
) -> Result<MultiStepAnswer>
where
    F: FnMut(&MultiStepLogEvent) + Send,  // ← Send境界追加
{
    // ...
}
```

**理由**:
- Axumのハンドラは`Send + 'static`なFutureを返す必要がある
- Web版はマルチスレッド環境で動作するため、クロージャが複数スレッド間で安全に移動できる必要がある
- CLI版（シングルスレッド）でも問題なく動作する

**実装場所**:
- `crates/app_core/src/openai/call/multi_step.rs` - Tool calling実装
- `crates/app_core/src/services/chat_service.rs` - ビジネスロジック層
- `crates/app_web/src/handlers/chat.rs` - Web APIハンドラ
- `crates/app_cli/src/modes/openai_chat.rs` - TUI実装

詳細は `feature/openai_web_tool.md` を参照してください。
