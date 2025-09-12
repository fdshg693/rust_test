# AI agent working notes for this repo (rust_test)

Purpose: Help an AI coding agent be productive immediately in this Rust TUI app that integrates OpenAI. Keep edits aligned with the patterns below and reference the cited files when extending features.

## Big picture
- TUI stack: ratatui + crossterm. No stdout logging while UI is active.
- OpenAI integration runs off the UI thread in a background worker using std::sync::mpsc channels.
- Logging uses tracing with daily-rotated file output in `logs/`; verbosity via `RUST_LOG`.
- Errors use color-eyre for friendly reports.

Key files
- `src/main.rs`: bootstrap. Installs color-eyre, loads `.env` (dotenvy), configures tracing to `logs/app.log` (no ANSI), then initializes/tears down ratatui and calls `rust_test::run`.
- `src/lib.rs`: app loop. Creates `App`, on each tick: `app.check_ai_response()`, `ui::render(...)`, then poll events with a 100ms timeout; dispatches to `event::handle_key`.
- `src/app.rs`: state. Holds input/last_submitted/ai_answer/pending/timers and the `tx`/`rx` channels. Starts the OpenAI worker in `with_config` and exposes small helpers (e.g., `submit_prompt`, `check_ai_response`).
- `src/event.rs`: key handling. Enter submits prompt; Esc/Ctrl+C exits; Backspace deletes; normal chars append.
- `src/openai.rs`: background worker. Spawns a thread, builds a Tokio runtime, uses `async-openai` Chat Completions. Two-step “function calling” flow with `get_constants` example.
- `src/ui.rs`: rendering. Layout panels: Guide, Input, Last Submitted, AI Answer, Footer (elapsed seconds). Uses `ratatui::widgets` and simple styling.
- `src/config.rs`: defaults (`model: gpt-4o-mini`, `max_tokens: 512`), plus example constants `X=42`, `Y=7` used by the OpenAI function.

## Data flow and boundaries
- UI thread (lib.rs): never blocks on network/IO. Sends prompts via `App.tx: Sender<String>` and polls `App.rx.try_recv()` each frame (`check_ai_response`).
- Worker thread (openai.rs): `rx_prompt.recv()` -> `process_prompt(...)` -> `tx_answer.send(String)` back to UI.
- Tracing targets: `app` (App lifecycle), `openai` (API calls and function results). Use these target names for new logs.

## Local dev workflow
- Run (debug or release):
  - `cargo run`
  - `cargo run --release`
- Env: `OPENAI_API_KEY` (read by async-openai). Loaded automatically from a root `.env` if present.
  - Windows PowerShell examples:
    - `$env:OPENAI_API_KEY = "sk-..."`
    - `$env:RUST_LOG = "openai=debug,app=info"`
- Logs: written to `logs/app.log.YYYY-MM-DD` (see `tracing_appender::rolling::daily`). Inspect logs instead of printing to stdout to avoid corrupting the TUI.
- Checks/tests: `cargo check` and `cargo test` (no tests yet).

## Conventions and patterns to follow
- Do not `println!` while ratatui is active. Use `tracing` with `info!/debug!/error!` and proper targets; output goes to files.
- Keep `main.rs` minimal; do terminal init/restore only here and delegate to `rust_test::run`.
- UI loop cadence is ~100ms (`crossterm_event::poll(Duration::from_millis(100))`). Don’t block; prefer channels and `try_recv`.
- Background tasks: spawn separate threads/runtimes and communicate via `std::sync::mpsc` (see `start_openai_worker`).
- Errors: return `color_eyre::Result` in entry points; propagate rather than unwrap in UI paths.

## Extension tips (with repo-specific examples)
- New key actions: add in `event::handle_key` and, if stateful, add fields/methods to `App` (e.g., mirror `submit_prompt`, `clear_input`).
- New UI panel: extend `ui::render` layout and add a `render_*` function similar to `render_ai_response`.
- More OpenAI tools: extend the `functions` vector in `openai::process_prompt` and implement a local function returning `serde_json::Value` (see `call_get_constants` using `config::X`/`Y`). Then include the function message in the second request as shown.
- Config-driven tweaks: defaults live in `config.rs`. The event loop currently hard-codes 100ms; wire `Config` through if you want it configurable.

## Gotchas
- Keep the tracing appender guard alive in `main.rs` (`let _keep_guard = guard;`) or logs may be lost.
- Always call `ratatui::restore()` on exit (already handled in `main.rs`). Avoid early returns that skip it.
- The UI thread must stay responsive; heavy work belongs in the worker thread.

If anything here is unclear or you need more detail (e.g., adding streaming responses, tests, or configurable polling), say what’s missing and I’ll refine this doc.
