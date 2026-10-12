# SSH Manager Configuration

The default directory is `~/.config/ssh-mgr/`. Each subdirectory is a group;
each `.toml` file can contain multiple `[[server]]` entries. Files are scanned
recursively. Press `r` in the manager after editing configuration.

The bundled hosts are examples. Replace their addresses and usernames before
connecting. Startup initializes this directory only when it does not exist.

## Environment Variables

To use the staging example's environment password:

```sh
cp ~/.config/ssh-mgr/env.example ~/.config/ssh-mgr/env
chmod 600 ~/.config/ssh-mgr/env
```

Edit `env` to replace the password placeholder. The manager loads this file at
startup; restart it after changes. Existing process environment variables take
precedence. `env.example` itself is not loaded.

## Server Configuration

```toml
[[server]]
name = "my-server"
host = "server.example.com"
port = 22
user = "deploy"
description = "Example server"

[server.password]
env = "SSH_MGR_STAGING_PASSWORD"
```

Choose one authentication source: `key = "~/.ssh/id_ed25519"`,
`env = "VARIABLE_NAME"`, `prompt = true`, or `value = "password"`.
Use private keys, not `.pub` files. `value` stores a plaintext password.
Keep real configuration and credentials out of source control.

Use `o` or `Enter` to enter SSH, and `O` to open a background tmux window.
Press `/` to search, then `Enter` to confirm the filter before using `o/O`.
