# Model-View-Update Approach

This project aspires to follow the model-view-update (MVU) pattern for all interactive flows. MVU keeps user-visible state changes predictable by routing every input through a single update function that produces a new model, while the view renders solely from that model, and side effects are isolated.

## Aspirations

- Keep mutations isolated to update handlers so every state transition is traceable.
- Treat views as pure renderers that derive their output from the model and immutable inputs.
- Funnel external events (OS signals, user input, timers, rebuild phases) into messages instead of letting them mutate state directly.
- Contain side effects inside commands or workers that report their results back as messages.

## How the Codebase Supports MVU Today

- **CLI rebuild flow** (`src/cli_mvu/mod.rs:52`): the `run_cli_loop` function receives all messages on channels and forwards them into `update`, keeping `Model` mutations centralized. Discovery, prompt, and build operations run on helper threads and only communicate by sending new `Msg` variants.
- **TUI event loop** (`src/tui/app/loop_runner.rs:114`): the terminal loop select-block forwards ticks, interrupts, and user input to `update_with_services`, and rendering happens immediately after from the current `App` state.
- **Handler modules** (`src/tui/app/handlers/events.rs:12`): message routing is delegated to focused modules (scan, rebuild, view picker, expansion). Each handler adjusts the `App` model directly while any long-running work is spawned in helper functions that send follow-up messages.
- **Background work isolation** (`src/tui/app/handlers/rebuild_worker.rs:17`): rebuild jobs execute on dedicated threads, emit progress through `Msg::RebuildJobOutput`, and signal completion so the UI never mutates state from outside the update path.
- **Pure views** (`src/tui/ui.rs`): drawing functions only read properties from `App` and CLI `Args`. They make no outbound calls and perform no mutations.
- **Table-building helpers** (`src/tui/app/rows.rs:5`): derived row data is produced through helper methods that operate on copies or on the current model but do not talk to services, keeping state preprocessing deterministic.
- **Tests reinforce MVU contracts** (`tests/test07_tui_rebuild.rs:48` and peers): integration tests interact with the TUI exclusively via `app::update_with_services`, mirroring the production update loop and preventing hidden mutation paths.

Maintaining these boundaries keeps it feasible to reason about state transitions and extend the application without unexpected interactions between user input, side effects, and rendering.
