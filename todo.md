# Production Readiness TODO

This document tracks the 25 items required to push Vyom (`tmux-music`) into 100/100 production readiness.

## Core Architecture (The Path to 100/100)
- [x] **1. The Reactive Render Loop (CPU Optimization)**
  - Stop the unconditional 60fps terminal.draw() loop in main.rs.
  - Make rendering dependent on `app.needs_redraw = true`.

- [x] **2. Configuration Fault Tolerance**
  - Stop wiping `.config/vyom/config.toml` upon TOML syntax errors.
  - Implement a persistent UI Toast error when parsing fails.

- [x] **3. Structured Logging Engine (Stderr Abatement)**
  - Integrate `tracing` and `tracing-appender` crates.
  - Delete all `eprintln!()` statements that corrupt TUI layouts.
  - Write background OS logs blindly to `~/.cache/vyom/vyom.log`.

- [x] **4. Ignored Errors Eradication**
  - Audit the 40+ `let _ = ...` instances throughout MPD logic.
  - Route those swallowed results to `tracing::warn!()`.

- [x] **5. Memory Leak / Clone Mitigation**
  - Refactor hot MPD polling loops.
  - Avoid massive string `.clone()`s using references/hashing where possible.

- [x] **6. Deployment Determinism**
  - Remove `Cargo.lock` from `.gitignore`.
  - Push the lockfile to git to guarantee version equality for end-users.

## Advanced Operations (The Path to Enterprise)
- [x] **7. CI/CD Multi-platform Builds**
  - Create a GitHub Action matrix for (x86_64 Mac, ARM64 Mac, Linux).
  
- [x] **8. Automated CI Testing Gate**
  - Block PRs globally unless `cargo test` and `cargo clippy -- -D warnings` pass cleanly.

- [x] **9. Telemetry / Crash Handlers**
  - Integrate `human-panic` panic hook overrides.
  - Format beautiful bug reports on crash instead of raw rust panics.

- [x] **10. macOS Code Signing & Notarization**
  - Automate the macOS Developer ID injection into the GitHub Action pipeline.

- [x] **11. Performance Benchmarking**
  - Add `cargo bench` tests using `criterion` to lock in DSP mathematical performance speeds.

## Deployment & Binary Optimization (Score: +6)
- [x] **12. Release Profile Optimization**
  - Add `[profile.release]` to `Cargo.toml` with `opt-level = 3`, `lto = "thin"`, `strip = true`, `codegen-units = 1`.

- [ ] **13. Version Automation**
  - Wire `cargo-release` or a CI step that auto-bumps version from a single source of truth (git tag).

- [x] **14. Fix Stale `.gitignore` Comment**
  - Remove the misleading comment about `Cargo.lock` being excluded (it is now tracked).

## Testing & Regression Safety (Score: +5)
- [x] **15. Unit Tests: Keybinding Engine**
  - Test `KeyConfig::matches()` for every branch: `Space`, `Enter`, `Backspace`, single chars, uppercase + Shift modifier.

- [x] **16. Unit Tests: EQ Math Functions**
  - Test `value_to_db()` / `db_to_value()` boundary values and roundtrip consistency.

- [x] **17. Unit Tests: Config Parsing**
  - Test that partial TOML (missing fields) correctly fills defaults via `#[serde(default)]`.
  - Test that an empty file produces valid defaults.

- [x] **18. Unit Tests: Toast Lifecycle**
  - Test `show_toast()`, deadline expiry in `on_tick()`, and stacking behavior.

- [x] **19. Unit Tests: Lyrics Timestamp Parsing**
  - Test `[mm:ss.xx]` line parsing with edge cases (malformed, missing timestamps, empty lines).

## Architecture Refinement (Score: +2)
- [ ] **20. Split `main.rs` into `app::run()`**
  - Extract the event loop, task spawning, and initialization into separate modules.
  - Target: `main.rs` under 100 lines.

- [ ] **21. Player Factory Pattern**
  - Introduce a `PlayerFactory` or builder to formalize controller/backend dispatch instead of ad-hoc `if/else` branching.

## Error Handling Hardening (Score: +1)
- [x] **22. Custom Error Types with `thiserror`**
  - Replace bare `anyhow` in internal modules with domain-specific error enums (`VyomError::MpdConnection`, `VyomError::ConfigParse`, `VyomError::AudioPipeline`).
  - Keep `anyhow` at the top-level `main()` boundary only.

## Concurrency Polish (Score: +1)
- [x] **23. Log Failed Channel Sends**
  - Replace the 22 remaining `let _ = tx.send(...)` with `if let Err(e)` + `tracing::debug!` so shutdown disconnections are diagnosable from log files.

## Terminal UX Polish (Score: +1)
- [x] **24. Explicit Resize Handler**
  - Add a match arm for `AppEvent::Input(Event::Resize(..))` that clears the terminal and forces a full redraw to prevent ghost artifacts.

## Code Polish (Score: +1)
- [x] **25. Clone Audit & `#[must_use]`**
  - Audit the 161 `.clone()` calls, specifically inside `AppEvent::Tick` and `TrackUpdate` handlers.
  - Add `#[must_use]` to pure functions like `value_to_db()`, `db_to_value()`.
