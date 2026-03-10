# Production Readiness TODO

This document tracks the 11 critical items required to push Vyom (`tmux-music`) into 100/100 Tier-1 enterprise production readiness.

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
