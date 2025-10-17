# Rust TUI Application - Multi-Mode System

This is a Rust TUI application using ratatui with a multi-mode system (Menu, OpenAI Chat, RPG Game).

## Project Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Main loop and mode-driven architecture
├── config.rs            # Configuration and constants
├── modes/               # Mode system (Phase 4 completed)
│   ├── mod.rs           # Mode trait and AppMode enum
│   ├── menu.rs          # Menu mode (startup screen)
│   ├── openai_chat.rs   # OpenAI chat mode
│   └── rpg_game.rs      # RPG game mode
├── openai/              # OpenAI API integration
│   ├── worker.rs        # Background worker
│   ├── simple.rs        # Simple API calls
│   └── call/            # Function calling system
├── rpg/                 # RPG game logic
├── sqlite/              # SQLite database utilities
└── bin/                 # Binary targets
```

## Features

### 1. Multi-Mode System (Phase 4 ✅)
- **Menu Mode**: Startup screen to select OpenAI Chat, RPG Game, or Exit
- **OpenAI Chat Mode**: Interactive chat with OpenAI's API
- **RPG Game Mode**: Text-based RPG adventure game
- Easy mode switching with Esc to return to menu

### 2. OpenAI Integration
- Background worker thread for API calls
- Streaming-like responses with real-time updates
- Function calling support (TAVILY_search, number guessing, etc.)
- Tool resolution with 2-step completion flow

### 3. RPG Game
- Character stats (HP, MP, Attack Power)
- Combat system with A (Attack), H (Heal), R (Rest), Q (Quit)
- Procedural enemy generation
- Win/lose conditions

### 4. SQLite Storage
- Simple key-value storage with file metadata
- API: `upsert_text`, `read_text`, `list_files`, etc.
- Supports both text and binary data

## Architecture (Phase 4)

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

### Old Files (Phase 4 cleanup)
- ❌ Deleted: `src/app.rs` → moved to `src/modes/openai_chat.rs`
- ❌ Deleted: `src/event.rs` → integrated into Mode::handle_key()
- ❌ Deleted: `src/ui.rs` → each mode has its own render() method

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

```bash
# Development mode
cargo run

# Release mode (optimized)
cargo run --release

# With logging
RUST_LOG=app=debug,openai=debug cargo run

# Check code
cargo check

# Run tests
cargo test
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
