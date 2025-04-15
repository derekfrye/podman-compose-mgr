# CLAUDE.md - Project Guidelines for podman-compose-mgr

## Build, Test & Lint Commands
- Build: `cargo build`
- Run: `cargo run -- [args]`
- Check: `cargo check`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`
- Test all: `cargo t` (limited to 2 parallel tests to prevent OOM)
- Test single: `cargo test --test test1` (Replace "test1" with specific test name)
- Test with output: `cargo test -j3 -- --nocapture --test-threads=3`
- Sequential tests: `cargo test -j3 -- --test-threads=3` (use if memory issues occur)

## Code Style Guidelines
- Use Rust 2024 edition
- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Use meaningful error types and proper error handling with Result
- Group imports by std lib, external crates, then internal modules
- Use the clap crate with derive feature for CLI argument parsing
- Prefer explicit type annotations for public interfaces
- Use Result<T, E> for functions that can fail, with descriptive errors
- Implement proper logging with error context
- Handle errors at appropriate levels, avoid unwrap() in production code
- Use the Default trait when appropriate for struct initialization

## Memory Considerations
- Tests are limited to 2 parallel executions to prevent OOM issues
- Mock clients are used for cloud storage when credentials aren't needed 
- Be cautious with large file operations, especially in CI environments

## Verbosity Levels
- Use `-v` or `--verbose` for basic informational messages (prefixed with "info:")
- Use `-v -v` or double `--verbose --verbose` for debug output (prefixed with "dbg:")
- Double verbose shows a copy-paste friendly command reconstruction
- Only essential messages are shown without verbose flags

## Cloud Storage Features
- Checks if files already exist in B2/R2 storage before uploading
- Warns users when overwriting existing files in cloud storage
- Shows creation and update timestamps when viewing file details
- Requires bucket names to be specified in JSON for B2/R2 uploads
