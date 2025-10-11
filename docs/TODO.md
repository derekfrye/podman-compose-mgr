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
