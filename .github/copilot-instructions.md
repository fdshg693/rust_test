# AI agent working notes for this repo (rust_test)

Purpose: Help an AI coding agent be productive immediately in this Rust TUI app with a multi-mode system (Menu, OpenAI Chat, RPG Game). Keep edits aligned with the patterns below and reference the cited files when extending features.

## Big picture
- TUI stack: ratatui + crossterm. No stdout logging while UI is active.
- Architecture: **Mode-driven** system where each mode (Menu, OpenAI Chat, RPG) is independent.
- OpenAI integration runs off the UI thread in a background worker using std::sync::mpsc channels.
- Logging uses tracing with daily-rotated file output in logs/; verbosity via RUST_LOG.
- Errors use color-eyre for friendly reports.

## Key files and structure

### Core loop
- src/main.rs: bootstrap. Installs color-eyre, loads .env (dotenvy), configures tracing to logs/app.log (no ANSI), then initializes/tears down ratatui and calls 
ust_test::run.
- src/lib.rs: **mode-driven app loop**. Starts with AppMode::Menu, on each tick: current_mode.update(), current_mode.render(...), then poll events with 100ms timeout; dispatches to current_mode.handle_key(). Mode transitions or exits are handled via AppMode.

### Mode system
- src/modes/mod.rs: defines Mode trait (update, render, handle_key) and AppMode enum (Menu, OpenAIChat, RpgGame, Exit).
- src/modes/menu.rs: **MenuMode** - startup menu with options to select OpenAI Chat, RPG Game, or Exit. Up/Down to select, Enter to confirm, Esc to exit.
- src/modes/openai_chat.rs: **OpenAIChatMode** - chat interface mode. Contains state from old App (input, last_submitted, ai_answer, pending, channels). Key handling: Enter to submit, Backspace to delete, chars to append. Esc to return to menu.
- src/modes/rpg_game.rs: **RpgGameMode** - RPG game mode. Displays game state, command input. A/H/R/Q for actions, Esc to return to menu.

### Shared utilities
- src/config.rs: defaults (model: gpt-4o-mini, max_tokens: 2000), plus example constants X=42, Y=7 used by the OpenAI function.
- src/openai/: background worker modules (worker.rs spawns thread, simple.rs/history.rs for API calls).
- src/rpg/: game rules, models, UI components for RPG mode.
- src/sqlite/: database utilities (shared by modes).

### Deleted files (Phase 4 cleanup)
-  src/app.rs  moved to src/modes/openai_chat.rs
-  src/event.rs  integrated into Mode::handle_key() implementations
-  src/ui.rs  each mode has its own 
ender() method

## Data flow and boundaries
- **UI thread (lib.rs)**: never blocks. Main loop: current_mode.update() (non-blocking), 	erminal.draw(|f| current_mode.render(f)), then poll(100ms) for key events. On key, calls current_mode.handle_key(key) which returns Option<AppMode> for transitions.
- **Mode-specific state**: Each mode holds its own state. OpenAIChatMode has channels (tx/rx) for worker communication. RPGGameMode has game state. MenuMode has selection index.
- **Worker thread (openai/worker.rs)**: spawned per OpenAIChatMode instance. Receives prompts via channel, processes via sync-openai, sends results back.
- **Tracing targets**: 
  - pp (app lifecycle, mode transitions)
  - openai (API calls and function results)
  - 
pg (game logic)
  - Use these for new logs.

## Local dev workflow
- Run (debug or release):
  - cargo run
  - cargo run --release
- Env: OPENAI_API_KEY (read by async-openai). Loaded automatically from a root .env if present.
  - Windows PowerShell examples:
    - \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 = "sk-..."
    - \ = "openai=debug,app=info"
- Logs: written to logs/app.log.YYYY-MM-DD (see 	racing_appender::rolling::daily). Inspect logs instead of printing to stdout to avoid corrupting the TUI.
- Checks/tests: cargo check and cargo test.

## Conventions and patterns to follow
- Do not println! while ratatui is active. Use 	racing with info!/debug!/error! and proper targets; output goes to files.
- Keep main.rs minimal; do terminal init/restore only here and delegate to 
ust_test::run.
- UI loop cadence is ~100ms (crossterm_event::poll(Duration::from_millis(100))). Don't block; prefer channels and 	ry_recv.
- **Mode trait methods must be non-blocking**:
  - update() - check for async results (e.g., check_ai_response()), update state, no I/O.
  - 
ender(\&self, f: \&mut Frame) - immutable rendering only.
  - handle_key(\&mut self, key: KeyEvent) -> Result<Option<AppMode>> - return next mode or None to continue.
- Background tasks: spawn separate threads/runtimes and communicate via std::sync::mpsc (see start_openai_worker in openai_chat.rs).
- Errors: return color_eyre::Result in entry points; propagate rather than unwrap in mode methods.

## Extension tips (with repo-specific examples)

### Adding a new mode
1. Create src/modes/my_mode.rs with MyMode struct and impl Mode.
2. Add variant to AppMode enum in src/modes/mod.rs.
3. Update AppMode::update(), AppMode::render(), AppMode::handle_key() in src/modes/mod.rs.
4. From menu or other modes, transition with 
eturn Ok(Some(AppMode::MyMode(MyMode::new()))).

### New key actions in existing mode
1. Edit OpenAIChatMode::handle_key() (or other mode's equivalent).
2. Add KeyCode pattern match and update self state or return next mode.
3. Example: Ctrl+A to clear input: KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => { self.input.clear(); }.

### New UI panel in a mode
1. Add a 
ender_* method to the mode struct (e.g., n render_debug_info(\&self, f: \&mut Frame, area: Rect)).
2. Call it from Mode::render() after layout split.
3. Example: self.render_debug_info(f, chunks[4]).

### More OpenAI tools
1. Extend the unctions vector in openai/call/mod.rs or openai/tools/mod.rs.
2. Implement a local handler function returning serde_json::Value (see call_get_constants or call_number_guess).
3. Include function message in second request as shown in multi-step flow.

## Gotchas
- Keep the tracing appender guard alive in main.rs (let _keep_guard = guard;) or logs may be lost.
- Always call 
atatui::restore() on exit (already handled in main.rs). Avoid early returns that skip it.
- The UI thread must stay responsive; heavy work belongs in worker threads.
- Mode transitions: returning Ok(Some(AppMode::Exit)) breaks the loop cleanly. Return Ok(None) to stay in current mode.
- If a mode spawns a worker (e.g., OpenAIChatMode), ensure cleanup on mode exit (drop channels, join thread, etc.). Currently handled implicitly when mode struct is dropped.

## Phase 4 Completion (October 2025)
-  Old files deleted: src/app.rs, src/event.rs, src/ui.rs
-  Mode system fully integrated in lib.rs
-  Documentation updated to reflect new architecture
- Next: Phase 5 - Add new features (AI RPG, settings mode, history persistence)
