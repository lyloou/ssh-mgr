# ssh-mgr

Terminal SSH manager with folder-based groups and fuzzy search.

## Run

```bash
cargo run -- --config ./examples/config
```

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
