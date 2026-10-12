# ssh-mgr

![Latest Release](https://img.shields.io/github/v/release/lyloou/ssh-mgr?color=green&label=Latest)

English | [简体中文](README-ZH.md)

**A keyboard driven SSH manager for tmux and terminal workflows.**

`ssh-mgr` turns a directory of small TOML files into a fast, searchable server tree. It launches the system OpenSSH client inside tmux, so interactive programs such as `vim`, `top`, and shells behave exactly as they do outside the manager.

## Why ssh-mgr

- Folder based groups that stay readable in Git.
- Fuzzy, letter by letter search across names, groups, hosts, users, descriptions, tags, and source paths.
- Native `ssh` inside tmux windows. No custom terminal emulator and no PTY translation layer.
- `o` for a foreground SSH window and `O` for a background SSH window.
- Password sources for environment variables, one time prompts, private keys, and explicit TOML values.
- Safe startup: missing default configuration is initialized with examples; existing files are never overwritten.

## Requirements

- Rust stable for building
- OpenSSH (`ssh`) on `PATH`
- tmux (`tmux`) on `PATH`

On macOS with Homebrew:

```bash
brew install tmux
```

## Install And Run

Install a prebuilt release for your OS and CPU architecture (OpenSSH and tmux are still required):

```bash
curl -fsSL https://raw.githubusercontent.com/lyloou/ssh-mgr/main/install.sh | bash
# Or select a version:
./install.sh 0.1.0
```

The installer places the binary in `~/.local/bin` and adds that directory to `.zshrc` or `.bashrc`. Reload the shell configuration before running `ssh-mgr`. Set `SSH_MGR_INSTALL_DIR` to choose another directory.

Build from source:

```bash
cargo build --release
./target/release/ssh-mgr
```

The binary automatically creates or attaches to the fixed tmux session `ssh-mgr` and runs the TUI in the `ssh-mgr` window. SSH connections become sibling windows in that session.

Useful commands:

```bash
ssh-mgr --help
ssh-mgr --version
ssh-mgr --config ./examples/config
```

The default configuration directory is `~/.config/ssh-mgr/`. It is created from the bundled examples on first launch only when it does not already exist.

## Screenshots

These screenshots were refreshed on 2026-10-12 from the release binary running in Ghostty with the bundled example configuration and `--lang en`. They show the current localized UI and keyboard shortcuts.

### Dashboard

![ssh-mgr dashboard in Ghostty](docs/screenshots/ghostty-dashboard.png)

The dashboard keeps the server tree, selected node details, action hints, and tmux context visible at the same time.

### Search

![ssh-mgr search in Ghostty](docs/screenshots/ghostty-search.png)

Search narrows the tree while preserving group context. Press `Enter` to confirm the filter, then use `o` or `O` on the selected server.

While typing a search, `j/k/h/l` are ordinary characters in the filter. Press `Enter` to finish typing and keep the filtered results; then `j/k` navigate and `h/l` collapse or expand groups. Press `/` to edit the filter again.

### Tree Navigation

![ssh-mgr collapsed group in Ghostty](docs/screenshots/ghostty-collapse.png)

The left and right arrow keys collapse or expand the selected group without losing the group row.

### Help

![ssh-mgr help overlay in Ghostty](docs/screenshots/ghostty-help.png)

The help overlay is available with `?` and closes with `Esc`.

## Language

Use `--lang en` (default) or `--lang zh`. `SSH_MGR_LANG` also selects the language; the CLI option takes precedence. Restart an existing manager window to apply the selection.

```bash
ssh-mgr --lang zh --config ./examples/config
```

## Configuration

Directories are groups. A TOML file may contain one or more `[[server]]` entries:

```text
~/.config/ssh-mgr/
├── production/
│   └── api.toml
└── staging/
    └── api.toml
```

Example:

```toml
[[server]]
name = "prod-api-01"
host = "10.0.0.11"
port = 22
user = "ubuntu"
description = "Production API primary"
tags = ["production", "api"]

[server.password]
key = "~/.ssh/id_ed25519"
```

### Authentication

Use exactly one authentication source inside `[server.password]`:

```toml
# Private key. OpenSSH handles a key passphrase in the tmux window.
[server.password]
key = "~/.ssh/id_ed25519"
```

```toml
# Password from the current process environment.
[server.password]
env = "SSH_MGR_PROD_PASSWORD"
```

```toml
# OpenSSH prompts interactively in the SSH window.
[server.password]
prompt = true
```

```toml
# Supported, but plaintext. Use only for local, tightly protected files.
[server.password]
value = "example-password"
```

For `env` and `value`, ssh-mgr writes a short lived, mode `0700` askpass helper in the system temporary directory. The password is not placed in the SSH command line, tmux window name, or application logs. The helper directory is removed when the SSH process exits.

## Local Environment File

At startup, ssh-mgr optionally loads `~/.config/ssh-mgr/env`, including when using `--config`. Existing process environment variables take precedence. The file is read again when the TUI starts inside tmux, so it also works with an existing tmux server. Restart the manager after changing this file; `r` reloads TOML only.

```dotenv
# ~/.config/ssh-mgr/env
SSH_MGR_PROD_PASSWORD='your password'
SSH_MGR_LANG=zh
```

Reference a variable in TOML:

```toml
[server.password]
env = "SSH_MGR_PROD_PASSWORD"
```

The optional file supports dotenv syntax, including comments, quoted values, and `export NAME=value`. It is parsed as data, not executed as a shell script. Protect credentials with `chmod 600 ~/.config/ssh-mgr/env`; do not commit this file. Malformed input fails startup without printing its contents.

## Keyboard Controls

The interface is organized as a compact workspace: the top bar shows the active search and tmux context, the left pane is the grouped server tree, the right pane contains details and actions, and the bottom bar reports the latest operation.

| Key | Action |
| --- | --- |
| `↑` / `↓`, `j` / `k` | Move through groups and servers |
| `←`, `h` | Collapse the selected group |
| `→`, `l` | Expand the selected group |
| `o` | Enter SSH in a new tmux window |
| `O` | Open SSH in the background and keep the TUI active |
| `Enter` | Confirm search, or alias for `o` in the list |
| `/` | Start fuzzy search |
| `Esc` | Clear search or close help |
| `r` | Rescan the configuration directory |
| `?` | Show the help overlay |
| `q` | Quit |

tmux itself remains available for switching sessions and windows:

```text
Ctrl-b n       next window
Ctrl-b p       previous window
Ctrl-b w       window picker
```

## Design And Safety Notes

- ssh-mgr delegates terminal behavior to the system OpenSSH client and tmux.
- It does not write to `~/.ssh/config` or alter private keys.
- Configuration files are read recursively and never modified by `r`.
- A malformed file stops the scan with its source path and server name in the error.
- The `value` mode is intentionally explicit because plaintext credentials can leak through file backups, permissions, or source control.

## Development

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

CI runs the same formatting, test, lint, and release build checks on every push and pull request.

Publish a GitHub release after updating `Cargo.toml` and `Cargo.lock` to the desired version:

```bash
./publish.sh             # uses Cargo.toml version
./publish.sh 0.1.0       # must match Cargo.toml
```

Requires Cargo, Rust, and an authenticated GitHub CLI (`gh`). The script builds for the current machine and uploads `target/archive/ssh-mgr-vX.Y.Z-<os>-<arch>.tar.gz`, containing the binary, installer, and examples. Supported platform labels are `macos`/`linux` and `aarch64`/`x86_64`; each release contains only the platform built on the publishing machine. Linux binaries require a compatible system runtime.

## Contributing

Small, focused pull requests are welcome. Please include a regression test for parser, search, or state behavior changes. Do not commit real hosts, credentials, private keys, or generated `target/` files.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the local check suite and [SECURITY.md](SECURITY.md) for credential handling and vulnerability reports. Release history is tracked in [CHANGELOG.md](CHANGELOG.md).

## License

Apache License 2.0. See [LICENSE](LICENSE).
