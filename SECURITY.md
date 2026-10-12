# Security Policy

## Reporting A Vulnerability

Please do not open a public issue for an undisclosed vulnerability. Contact the maintainers privately with:

- a short description of the issue;
- affected versions or commits;
- reproduction steps;
- any known mitigation.

Until a private reporting address is published, use a private GitHub security advisory for the repository or contact the repository owner directly.

## Credential Handling

- Private key paths are passed to the system `ssh` client.
- `prompt = true` delegates password entry to OpenSSH inside tmux.
- `env` and `value` use a temporary mode `0600` password file and a mode `0700` askpass helper.
- Temporary credential files are removed by the SSH wrapper when the process exits.
- `value` stores plaintext credentials in TOML and should not be used in shared repositories.
- Never commit real credentials, private keys, or production configuration.
