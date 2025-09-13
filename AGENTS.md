# AGENTS.md - Agentic Coding Guidelines for RT Terminal Emulator

## Build, Lint, and Test Commands
- Build: `cargo build` or `cargo build --release`
- Run: `cargo run` or `cargo run --release`
- Test all: `cargo test`
- Test single: `cargo test <testname>`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Code Style Guidelines
- **Imports:** Group std, external, and internal; use explicit imports.
- **Formatting:** Run `cargo fmt` before commit; follow Rust 2021 edition.
- **Naming:** camelCase for variables/functions, PascalCase for types, SCREAMING_SNAKE_CASE for constants.
- **Types:** Prefer explicit types; avoid `unwrap`/`expect` in production code.
- **Error Handling:** Use `Result`/`Option`, propagate errors with `?`, log errors for audit/compliance.
- **Comments:** Document public APIs and complex logic; keep comments concise.
- **Testing:** Write unit and integration tests; aim for >90% coverage; use `cargo test <testname>` for single tests.
- **Documentation:** Maintain inline docs and update README.md for major changes.
- **Peer Review:** All changes require code review before merge.
- **Versioning:** Use semantic versioning for releases.

No Cursor or Copilot rules detected. Update this file if such rules are added.