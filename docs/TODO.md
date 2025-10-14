# Refactoring Targets

## Files over 200 LOC
- src/tui/ui.rs — target: split or extract so each file stays under 200 LOC.
- src/cli_mvu/mod.rs — refactor into smaller modules to stay under 200 LOC.
- src/tui/app/state.rs — reduce complexity to stay under 200 LOC.
- src/image_build/buildfile_helpers/prompt_helpers.rs — break up logic to stay under 200 LOC.
- src/utils/path_utils.rs — reorganize helpers so the file stays under 200 LOC.
- src/tui/app/handlers/rebuild.rs — extract reusable pieces to stay under 200 LOC.
- src/tui/app/handlers/rebuild_worker.rs — split worker logic to stay under 200 LOC.
- src/read_interactive_input/helpers.rs — trim helper set to stay under 200 LOC.

## Functions, methods, and impls over 50 LOC
- src/image_build/buildfile_helpers/discovery_helpers.rs: find_buildfile (~52 LOC) — extract reusable helpers to shorten.
- src/image_build/rebuild/grammar.rs: build_rebuild_grammars (~63 LOC) — simplify grammar assembly.
- src/image_build/rebuild/interaction.rs: read_val_loop (~52 LOC) — streamline read loop.
- src/tui/app/handlers/rebuild_worker.rs:
  - CommandHelper for TuiCommandHelper::exec_cmd (~62 LOC) — break into smaller commands.
  - impl CommandHelper for TuiCommandHelper (~77 LOC) — pull out shared helpers.
- src/tui/app/keymap.rs: map_keycode_to_msg (~57 LOC) — decompose key mapping.
- src/tui/ui.rs: draw_work_queue (~56 LOC) — split drawing routines.
- src/tui/app/rows.rs: impl App (~155 LOC) — move row helpers into dedicated module.

## Rebuild queue dedupe gaps
- Container view emits one row per `DiscoveredImage`, keyed by `(image, container, source_dir)`, so identical tags sourced from different directories remain (`src/infra/discovery_adapter.rs:64`).
- When users check multiple rows we enqueue a `RebuildJobSpec` for each without deduping (`src/tui/app/handlers/rebuild.rs:223`), so the queue can run the same `podman build -t <image>` repeatedly.
- Parallelizing the rebuild worker would trigger concurrent builds for the same tag, so we need a pre-queue dedupe (by image/tag) or smarter scheduling in `src/tui/app/handlers/rebuild_worker.rs`.

## Low-hanging perf opportunities
- `AppCore` stores trait objects (`Arc<dyn DiscoveryPort>`, `Arc<dyn PodmanPort>` at `src/app/mod.rs:18`); replacing with generics would remove the vtable hop in hot discovery/detail paths.
- `CommandHelper::exec_cmd` returns `Result<(), Box<dyn std::error::Error>>` (`src/interfaces.rs:21`); adopting a concrete error type eliminates heap boxing for command failures.
- Command utilities expose boxed errors (`src/utils/cmd_utils.rs:33` et al.), which feed production code; rework to return `PodmanComposeMgrError` or another concrete enum.
- `PrintFunction` is `Box<dyn Fn>` (`src/read_interactive_input/types.rs:22`); if we only register static printers, switching to plain function pointers or generics avoids the allocation per handler.
- `build_rows_for_image_view` clones strings into a `HashSet<String>` (`src/tui/app/rows.rs:55`); we can dedupe by borrowing `&str` from `DiscoveredImage` to skip transient heap churn when rebuilding the list.

## Error handling follow-ups
- Standardize error propagation in the rebuild queue; avoid the `Result<(), String>` shim in `src/tui/app/handlers/rebuild_worker.rs:61` so callers can surface structured `PodmanComposeMgrError` values.
- Replace `unwrap()`/`expect()` usage around file discovery (`src/image_build/buildfile_helpers/discovery_helpers.rs:15`, `src/image_build/buildfile_build.rs:34`) with fallible conversions and contextual errors to prevent panics on missing or non-UTF-8 paths.
- Remove the direct `std::process::exit(0)` in `src/image_build/rebuild/interaction.rs:135` and return a typed error so the TUI can shut down gracefully on Ctrl+C without terminating the host process.
- Stop double-wrapping/formatting errors when launching builds (`src/image_build/buildfile/mod.rs:66`, `src/image_build/buildfile_build.rs:57`); rely on a single error enum or `thiserror` chain and let the UI handle user-facing messaging.

## selectable text in the rebuild view
Using a mouse and trying to highlight lines within rebuild view doesn't seem to work.

## greppable text in the rebuild view
We should support `/` and `?` to search through the buffer, highlighting matches. And it should support regex.