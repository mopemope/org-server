# Repository Guidelines

## Project Structure & Module Organization
The workspace contains two crates: `parser/` provides the Org-mode parsing library with Pest grammar (`parser/org.pest`) plus integration tests and fixtures under `parser/tests/` and `parser/test_data/`; `server/` hosts the CLI and HTTP server entry point in `server/src/main.rs` with feature modules such as `cli.rs` and `json_output.rs`. Shared workspace configuration and dependency pins live in the root `Cargo.toml`, while `rustfmt.toml` and `README.md` document formatting and usage expectations.

## Build, Test, and Development Commands
Use `cargo check --workspace` for a fast lint of compilation errors, `cargo build --workspace` for full builds, and `cargo build --release` when benchmarking or packaging binaries. Run `cargo test --workspace` to execute unit, integration, and parser regression tests; add `-- --nocapture` when debugging. Developer tooling includes `cargo fmt --all` for formatting and `cargo clippy --workspace --all-targets` to catch lint issues before review.

## Coding Style & Naming Conventions
All Rust sources target edition 2024 with 4-space indentation per `rustfmt.toml`; always run `cargo fmt --all` before committing. Follow idiomatic Rust naming: modules and functions in `snake_case`, types and traits in `PascalCase`, constants in `SCREAMING_SNAKE_CASE`. Keep module files under 300 lines when practical, splitting helpers into submodules (e.g., `parser/src/json_conversion.rs`) to maintain clarity. Prefer `tracing` macros for logging and surface errors via `anyhow::Result` or `thiserror`-derived enums.

## Testing Guidelines
Add focused unit tests near implementation files and higher-level scenarios in `parser/tests/` or `server/tests/` (create the latter as needed). Mirror fixture inputs in `parser/test_data/` when covering edge cases like deeply nested timestamps. Name test functions descriptively (`test_extract_deadlines_in_future`) and ensure new behaviour is exercised through `cargo test --workspace`. When introducing background tasks, include async tests or mocked timers to prevent flakiness.

## Commit & Pull Request Guidelines
Commits follow the conventional `type(scope): summary` style (e.g., `feat(reminders): add expired reminder handling`) with imperative summaries under 72 characters. Keep each commit focused on one logical change and include relevant tests. Pull requests should explain the motivation, enumerate major changes, mention affected crates, and link issues or discussion threads. Provide CLI examples or configuration snippets when altering user-facing behaviour, and add screenshots for any UI or API contract changes.

## Security & Configuration Tips
Avoid committing real Org files or secrets; rely on redacted samples under `parser/test_data/`. Validate configuration paths via `shellexpand` and boundary-check reminder intervals to prevent panics. When enabling network or filesystem features, document new environment variables in `README.md` and note required capabilities in the PR description.
