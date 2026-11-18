#!/bin/sh
# One-script installer for flashbrowse.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/nhrbuilds/flashbrowse/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/nhrbuilds/flashbrowse/main/install.sh | sh -s -- --version v0.1.0
#   curl -fsSL .../install.sh | INSTALL_DIR=/tmp/bin sh
set -eu

REPO="nhrbuilds/flashbrowse"
VERSION="${1:-}"
if [ "$VERSION" = "--version" ] || [ "$VERSION" = "-v" ]; then
  shift
  VERSION="${1:-}"
fi
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

os="$(uname -s)"
arch="$(uname -m)"
case "$os-$arch" in
  Linux-x86_64) ASSET="flashbrowse-linux-x86_64" ;;
  Linux-aarch64|Linux-arm64) ASSET="flashbrowse-linux-aarch64" ;;
  Darwin-arm64) ASSET="flashbrowse-macos-aarch64" ;;
  Darwin-x86_64) ASSET="flashbrowse-macos-x86_64" ;;
  *) echo "Unsupported platform: $os/$arch - build from source: git clone https://github.com/$REPO && cargo build --release" >&2; exit 1 ;;
esac

# Resolve tag (latest by default via GitHub redirect).
if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest" | sed 's/.*releases\/tag\///')"
fi
TAG="$VERSION"
# Normalise: releases use tags like v0.1.0.
case "$TAG" in v*) ;; *) TAG="v$TAG" ;; esac

URL="https://github.com/$REPO/releases/download/$TAG/$ASSET.tar.gz"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "-> Downloading $ASSET ($TAG) ..."
curl -fsSL "$URL" -o "$TMPDIR/$ASSET.tar.gz"
tar -xzf "$TMPDIR/$ASSET.tar.gz" -C "$TMPDIR"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMPDIR/flashbrowse" "$INSTALL_DIR/flashbrowse"

# macOS: clear the quarantine flag on the binary we just installed. The
# attribute is only set when a file is downloaded through a browser; curl
# does not set it, but users who fetch the tarball by hand and then run this
# script would hit Gatekeeper without this.
if [ "$os" = "Darwin" ]; then
  xattr -dr com.apple.quarantine "$INSTALL_DIR/flashbrowse" 2>/dev/null || true
fi

echo "Installed: $INSTALL_DIR/flashbrowse ($TAG)"
echo "Add to PATH if needed:  export PATH=\"$INSTALL_DIR:\$PATH\""
echo "Then:"
echo "  flashbrowse setup            # downloads managed Chrome (~120MB, once)"
echo "  flashbrowse open https://example.com"
echo "  flashbrowse mcp              # for Claude Code / Cursor / Codex"
