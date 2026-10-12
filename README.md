# ssh-mgr

Terminal SSH manager with folder-based groups and fuzzy search.

## Run

```bash
cargo run -- --config ./examples/config
```

SSH sessions are launched by the system `ssh` command inside tmux windows. Running the binary outside tmux automatically creates or attaches to the fixed `ssh-mgr` session and runs the TUI in the `ssh-mgr` window:

```bash
tmux new-session -A -s ssh-mgr ./target/release/ssh-mgr
```

`o` creates and switches to an SSH window. `O` creates an SSH window in the background and keeps the TUI active. `Enter` remains as an alias for foreground SSH. The TUI always remains in the `ssh-mgr` window.

Without `--config`, the application scans `~/.config/ssh-mgr/` recursively for `.toml` files. If that directory does not exist, the bundled example configuration is copied there once. Existing files are never overwritten. Each directory is a group and each file may contain one or more `[[server]]` entries.

## Configuration

Authentication is unified under `[server.password]`:

```toml
[server.password]
key = "~/.ssh/id_ed25519"
```

Use an environment variable or prompt instead:

```toml
[server.password]
env = "SSH_MGR_PASSWORD"
```

```toml
[server.password]
prompt = true
```

`value = "..."` is supported for compatibility, but stores a plaintext password and should be avoided for shared configuration.

For `env` and `value`, ssh-mgr creates a short-lived `SSH_ASKPASS` helper and passes the password only to the SSH child process. The temporary files are removed when SSH exits. `prompt = true` leaves password prompting to the system SSH client.
