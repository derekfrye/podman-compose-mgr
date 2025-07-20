# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

podman-compose-mgr is a Rust CLI tool that manages docker-compose deployments and secrets across multiple cloud storage providers. It operates in five distinct modes: Rebuild (default container management), SecretRetrieve, SecretInitialize, SecretUpload, and SecretMigrate.

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

## Architecture Patterns

### Dependency Injection
- Uses trait-based interfaces for testability: `CommandHelper`, `ReadInteractiveInputHelper`, `AzureKeyVaultClient`, `R2StorageClient`
- Default implementations for production, mockable versions for testing
- Enables comprehensive unit testing without external dependencies

### Error Handling
- Custom error types using `thiserror` crate for each major module
- Consistent `Result<T, E>` patterns throughout codebase
- Graceful error propagation with contextual messages

### Cloud Storage Abstraction
- Unified interface for Azure KeyVault, AWS S3, and Cloudflare R2
- Hash-based file deduplication across all storage backends
- Graceful fallback to mock clients when credentials unavailable

### Interactive Input System
- Grammar-based input parsing in `read_interactive_input/`
- Flexible prompt generation with terminal width detection
- User-friendly input validation and formatting

## Key Modules

### Core Application Flow
- `main.rs`: Entry point with global Ctrl+C handling
- `lib.rs`: Main orchestrator (`run_app`) routing to mode handlers
- `walk_dirs.rs`: Directory traversal with regex-based filtering

### Secrets Management (`secrets/`)
- Multi-provider cloud storage integration (Azure/AWS/R2)
- Migration capabilities between storage backends
- JSON-based configuration with comprehensive validation

### Container Management (`image_build/`)
- Docker Compose YAML parsing and service extraction
- Podman integration for image building and pulling
- Dependency injection for testing container operations

### Terminal Interface (`tui/`)
- Ratatui-based terminal UI for interactive operation
- Event handling and user input processing

## Testing Guidelines
- Use trait-based mocking with `mockall` crate
- Mock cloud storage clients when credentials unavailable
- Test execution limited to prevent memory issues in CI environments
