#!/usr/bin/env bash
# ssh-mgr installer
#
# Usage:
#   ./install.sh [version]
#   curl -fsSL https://raw.githubusercontent.com/lyloou/ssh-mgr/main/install.sh | bash

set -euo pipefail

REPO="lyloou/ssh-mgr"
INSTALL_DIR="${SSH_MGR_INSTALL_DIR:-$HOME/.local/bin}"

log() { printf '[ssh-mgr] %s\n' "$*"; }
fail() { printf '[ssh-mgr] error: %s\n' "$*" >&2; exit 1; }

resolve_latest_version() {
  local response
  response="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")" || return 1
  if command -v jq >/dev/null 2>&1; then
    jq -r '.tag_name // empty' <<<"$response"
  else
    sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' <<<"$response" | head -n 1
  fi
}

VERSION="${1:-}"
if [[ "$VERSION" == --help || "$VERSION" == -h ]]; then
  printf 'Usage: %s [version]\nInstalls the latest release by default to ~/.local/bin.\n' "$0"
  exit 0
fi
[[ $# -le 1 ]] || fail "usage: $0 [version]"
if [[ -z "$VERSION" ]]; then
  log "fetching latest release"
  VERSION="$(resolve_latest_version)" || fail "could not query GitHub releases"
fi
[[ "$VERSION" == v* ]] || VERSION="v${VERSION}"
[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "invalid version: $VERSION"

case "$(uname -s)" in
  Darwin) OS="macos" ;;
  Linux) OS="linux" ;;
  *) fail "unsupported operating system" ;;
esac
case "$(uname -m)" in
  arm64|aarch64) ARCH="aarch64" ;;
  x86_64|amd64) ARCH="x86_64" ;;
  *) fail "unsupported CPU architecture" ;;
esac
ARCHIVE_URL="https://github.com/${REPO}/releases/download/${VERSION}/ssh-mgr-${VERSION}-${OS}-${ARCH}.tar.gz"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

log "downloading ${ARCHIVE_URL}"
curl -fsSL "$ARCHIVE_URL" -o "$TMP_DIR/ssh-mgr.tar.gz" || fail "download failed for ${VERSION}"
tar -xzf "$TMP_DIR/ssh-mgr.tar.gz" -C "$TMP_DIR"

[[ -f "$TMP_DIR/ssh-mgr" ]] || fail "release archive does not contain ssh-mgr"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMP_DIR/ssh-mgr" "$INSTALL_DIR/ssh-mgr"
log "installed ${VERSION} to ${INSTALL_DIR}/ssh-mgr"

USER_SHELL="${SHELL:-}"
case "${USER_SHELL##*/}" in
  zsh) SHELL_RC="$HOME/.zshrc" ;;
  bash) SHELL_RC="$HOME/.bashrc" ;;
  *) SHELL_RC="" ;;
esac
PATH_LINE="export PATH=\"$INSTALL_DIR:\$PATH\""
if [[ -n "$SHELL_RC" ]] && ! grep -Fqx "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
  printf '\n# ssh-mgr\n%s\n' "$PATH_LINE" >> "$SHELL_RC"
  log "added ${INSTALL_DIR} to ${SHELL_RC}"
fi

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  if [[ -n "$SHELL_RC" ]]; then
    log "run: source $SHELL_RC"
  else
    log "add ${INSTALL_DIR} to your shell's PATH"
  fi
fi
log "run: ssh-mgr --help"
