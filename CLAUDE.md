# CLAUDE.md - Project Guidelines for podman-compose-mgr

## Build, Test & Lint Commands
- Build: `cargo build`
- Run: `cargo run -- [args]`
- Check: `cargo check`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`
- Test all: `cargo test`
- Test single: `cargo test test1` (Replace "test1" with specific test name)
- Test with output: `cargo test -- --nocapture`

## Code Style Guidelines
- Use Rust 2021 edition
- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use meaningful error types and proper error handling with Result
- Group imports by std lib, external crates, then internal modules
- Use the clap crate with derive feature for CLI argument parsing
- Prefer explicit type annotations for public interfaces
- Use Result<T, E> for functions that can fail, with descriptive errors
- Implement proper logging with error context
- Handle errors at appropriate levels, avoid unwrap() in production code
- Use the Default trait when appropriate for struct initialization