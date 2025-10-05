# Repository Guidelines

## Project Structure & Module Organization
- `src/` — Rust sources. Entrypoint `main.rs`; library in `lib.rs`.
  - `args/` (CLI parsing/validation), `utils/` (logging, paths, podman helpers), `walk_dirs.rs`, `image_build/`, `tui/`.
- `tests/` — integration tests (spawn the binary, OS‑specific behavior). Example: `tests/test1_walk_dirs_varying_term_size.rs`.
- `doc/README.md` — user docs; `scripts/` — utility scripts; `.trunk/` — lint/format config.

## Build, Test, and Development Commands
- Build: `cargo build` (debug) or `cargo build --release`.
- Run: `cargo run -- --help` or e.g. `cargo run -- --path ~/docker -e "docker/archive" --build-args USERNAME=$(id -un)`.
- Test: `cargo test` (use `-- --nocapture` to see stdout). CI/PRs should pass build, tests, clippy, and fmt.
- Lint/Format: `cargo fmt --all` and `cargo clippy -- -D warnings`. If you use Trunk, `trunk check` matches `.trunk/trunk.yaml`.

## Coding Style & Naming Conventions
- Rust 2024 edition. Use rustfmt defaults (4‑space indent). Run `cargo fmt --all` before committing.
- Naming: modules/files `snake_case`; types/traits `UpperCamelCase`; functions/vars `snake_case`; constants `SCREAMING_SNAKE_CASE`.
- Keep modules cohesive (follow existing layout under `src/`); prefer `Result<T, E>` and error types in `errors.rs`/`thiserror`.

## Testing Guidelines
- Framework: standard Cargo tests. Prefer integration tests in `tests/` for CLI flows; unit tests inline with `#[cfg(test)]` when practical.
- Naming: descriptive filenames like `testNN_description.rs` (see existing `test11_ctrlc.rs`).
- Platform notes: some tests simulate Ctrl+C and spawn the binary; ensure they run non‑interactively. Windows paths are gated via dev‑deps.

## Commit & Pull Request Guidelines
- Commits: small, focused, imperative subject (e.g., "split up a large file", "fmt", "bump version").
- PRs: include what/why, notable design choices, and any behavioral changes. Link issues. Include sample commands/output or screenshots for TUI changes.
- Required: build, tests, clippy, and fmt must pass; avoid introducing new warnings.

## Security & Configuration Tips
- Runtime expects Podman available on PATH; commands shell out to `podman`. Developing on Linux/macOS is recommended; Windows is supported via conditional deps.
