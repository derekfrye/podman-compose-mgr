# Repository Guidelines

## Project Structure & Module Organization
- `src/` — Rust sources. Entrypoint `main.rs`; reusable library items in `lib.rs`.
  - `app/`, `cli_mvu/`, and `mvu/` — MVU state machine, CLI-specific shell, and shared MVU primitives.
  - `args/` — CLI argument parsing and validation helpers.
  - `domain/`, `infra/`, `ports.rs`, `interfaces.rs` — Domain abstractions plus adapters for Podman, filesystem, and time services.
  - `image_build/` — Build command orchestration and job queue logic.
  - `read_interactive_input/` — Facilities for prompting the user outside the TUI.
  - `testing/` — Test fixtures and utilities reusable across unit and integration tests.
  - `tui/` — Terminal UI components.
  - `utils/` and `walk_dirs.rs` — Logging, path helpers, Podman command wrappers, and directory traversal.
- `tests/` — Integration tests that spawn the binary. Current focus is on TUI flows (`tests/test02_tui.rs`–`tests/test20_cli_prompt_format.rs`) with supporting fixtures in sibling directories (e.g., `tests/test07/`, `tests/test9/`).
- `docs/` — User and architecture documentation (`docs/README.md`, `docs/MVU.md`, `docs/TODO.md`).
- `mock_podman/` — Test doubles for Podman interactions.
- `target/` — Cargo build output (ignored by git).

## Build, Test, and Development Commands
- Build: `cargo build` (debug) or `cargo build --release`.
- Run: `cargo run -- --help` or e.g. `cargo run -- --path ~/docker -e "docker/archive" --build-args USERNAME=$(id -un)`.
- Test: `cargo test` or `cargo nextest run` (use `-- --nocapture` to see stdout). CI/PRs must pass build, tests, `cargo clippy -- -D warnings`, and `cargo fmt --all`.

## Coding Style & Naming Conventions
- Rust 2024 edition. Use rustfmt defaults (4-space indent). Always run `cargo fmt --all` before committing.
- Naming: modules/files `snake_case`; types/traits `UpperCamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- Keep modules cohesive and prefer expressing cross-module contracts via traits in `interfaces.rs`/`ports.rs`. Use `Result<T, E>` with error types from `errors.rs` or `thiserror`.

## Testing Guidelines
- Use standard Cargo tests. Integration tests in `tests/` exercise CLI/TUI behavior by spawning the binary and simulating user input. Add unit tests inline with `#[cfg(test)]` when practical.
- Follow existing naming (`testNN_description.rs`) and reuse fixtures in `tests/` subdirectories or `src/testing/`.
- Tests should run non-interactively on Linux/macOS; gate Windows-specific paths with conditional compilation if needed.

## Commit & Pull Request Guidelines
- Commits: small, focused, imperative subject lines (e.g., "refine tui rebuild queue", "fmt").
- PRs: describe what/why, highlight notable design choices or behavioral changes, and link issues. Include sample commands/output or screenshots for TUI changes.
- Required: build, tests, clippy, and fmt must pass; avoid introducing new warnings.

## Security & Configuration Tips
- Runtime expects Podman on the `PATH`; commands shell out to `podman`. Developing on Linux/macOS is recommended; Windows support exists via conditional dependencies.
