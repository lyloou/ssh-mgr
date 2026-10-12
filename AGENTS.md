# Repository Guidelines

## Project Structure & Module Organization

This Rust 2021 executable uses Ratatui and Crossterm. `src/main.rs` handles startup, CLI options, and the fixed `ssh-mgr` tmux session. `src/tui.rs` implements rendering, navigation, and SSH window launching. Configuration models and recursive TOML scanning live in `src/config/`; search, localization, and dotenv loading live in `src/search.rs`, `src/i18n.rs`, and `src/env_file.rs`.

Unit tests reside alongside implementation code in `#[cfg(test)]` modules. Example configurations are in `examples/config/`; documentation screenshots are in `docs/screenshots/`. Keep `README.md` and `README-ZH.md` consistent. CI is defined in `.github/workflows/ci.yml`; installation and publishing scripts are at the repository root.

## Build, Test, and Development Commands

- `cargo run -- --config ./examples/config --lang en`: run with sample hosts.
- `cargo build --release`: build `target/release/ssh-mgr`.
- `cargo test --all-targets`: run the Rust test suite.
- `cargo fmt --all -- --check`: check formatting.
- `cargo clippy --all-targets --all-features -- -D warnings`: reject lint warnings.
- `git diff --check`: detect whitespace errors before committing.

Interactive execution requires OpenSSH and tmux on `PATH`. Use ordinary Cargo commands; use mise only when runtime installation or version selection is needed.

## Coding Style & Naming Conventions

Use rustfmt defaults, four-space indentation, `snake_case` for functions and modules, and `PascalCase` for types. Follow existing error handling with `anyhow` and descriptive context. Keep changes focused; avoid unrelated refactors. Route UI text through the existing localization mechanism.

## Testing Guidelines

Use Rust's built-in `#[test]` framework and descriptive behavior names, such as `rejects_conflicting_password_sources`. Add regression coverage for parsing, search, grouping, authentication, and terminal-state changes. No numeric coverage threshold is defined. Use disposable temporary directories and clean them up. Run the CI checks above; describe any manual tmux validation separately.

## Commit & Pull Request Guidelines

History uses short imperative subjects, such as `Add localization and dotenv environment loading`. PRs should explain the problem, resulting behavior, and validation. Link related issues when available; include updated screenshots for visible UI changes.

## Security & Agent Instructions

For requested Ghostty screenshot updates, use `osascript` to activate and operate the window and `screencapture -x -o -l WINDOW_ID OUTPUT.png` to capture only that window. Save images in `docs/screenshots/`, inspect every image for the intended UI state and unobstructed content, and update README references when needed. Use bundled examples rather than private server configurations.

Delegate SSH terminal behavior to OpenSSH and tmux. Never commit credentials, private keys, real host configuration, or `target/`. Preserve existing configuration during initialization and reload. Consult `SECURITY.md` for credential handling. State verification steps before implementation; use CLI checks by default and GUI testing only when requested. Do not publish releases or modify unrelated user files without authorization.
