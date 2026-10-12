#!/usr/bin/env bash
# Build a release archive and publish it to GitHub.
#
# Usage:
#   ./publish.sh         # defaults to Cargo.toml version
#   ./publish.sh 0.1.0    # must match Cargo.toml
#   ./publish.sh v0.1.0

set -euo pipefail

REPO="lyloou/ssh-mgr"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="${1:-}"

fail() { printf '[ssh-mgr] error: %s\n' "$*" >&2; exit 1; }

if [[ "$VERSION" == --help || "$VERSION" == -h ]]; then
  printf 'Usage: %s [version]\nBuilds for the current OS/architecture; defaults to Cargo.toml version.\n' "$0"
  exit 0
fi
[[ $# -le 1 ]] || fail "usage: $0 [version]"
PACKAGE_VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$PROJECT_DIR/Cargo.toml" | head -n 1)"
VERSION="${VERSION:-$PACKAGE_VERSION}"
VERSION="${VERSION#v}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "invalid version: ${VERSION}"
[[ "$VERSION" == "$PACKAGE_VERSION" ]] || fail "update Cargo.toml to ${VERSION} first (currently ${PACKAGE_VERSION})"

command -v cargo >/dev/null 2>&1 || fail "cargo is required"
command -v rustc >/dev/null 2>&1 || fail "rustc is required"
command -v gh >/dev/null 2>&1 || fail "GitHub CLI (gh) is required"
command -v tar >/dev/null 2>&1 || fail "tar is required"

cd "$PROJECT_DIR"
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
echo "=== ssh-mgr publish v${VERSION} ==="
HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
cargo generate-lockfile
cargo build --release --locked --target "$HOST_TARGET"

BUILD_BINARY="$PROJECT_DIR/target/$HOST_TARGET/release/ssh-mgr"
[[ -x "$BUILD_BINARY" ]] || fail "release binary was not produced: $BUILD_BINARY"

ARCHIVE_DIR="$PROJECT_DIR/target/archive"
STAGE_DIR="$PROJECT_DIR/target/publish-stage"
ARCHIVE_PATH="$ARCHIVE_DIR/ssh-mgr-v${VERSION}-${OS}-${ARCH}.tar.gz"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR" "$ARCHIVE_DIR"
install -m 0755 "$BUILD_BINARY" "$STAGE_DIR/ssh-mgr"
cp -R "$PROJECT_DIR/examples" "$STAGE_DIR/examples"
install -m 0755 "$PROJECT_DIR/install.sh" "$STAGE_DIR/install.sh"
tar --exclude='.DS_Store' --exclude='*/.DS_Store' \
  -czf "$ARCHIVE_PATH" -C "$STAGE_DIR" ssh-mgr install.sh examples

gh release create "v${VERSION}" \
  --repo "$REPO" \
  --title "ssh-mgr v${VERSION}" \
  --notes "Release v${VERSION}" \
  "$ARCHIVE_PATH"

echo "Release: https://github.com/${REPO}/releases/tag/v${VERSION}"
