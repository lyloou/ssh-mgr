# Contributing

Thanks for improving ssh-mgr.

## Before Opening A Pull Request

Run the same checks used by CI:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Keep changes focused and add a regression test when changing parsing, search, grouping, or authentication behavior. Do not include real server addresses, credentials, private keys, or local tmux state in commits.

## Commit Messages

Use a short imperative subject, for example:

```text
Improve grouped server search
Reject conflicting password sources
```

## Scope

The application delegates interactive SSH behavior to OpenSSH and tmux. Changes that implement another terminal protocol should include a clear reason and an end to end test plan.
