#!/usr/bin/env bash
# llmwiki-cli installer — modeled on rustup/kimi-code installer.
# Usage: curl -LsSf https://github.com/<owner>/llmwiki/raw/main/install.sh | bash

# TODO(release): sed-replace "<owner>" with the real GitHub owner (fg)
# before tagging v0.3.0. The spec explicitly ships with the placeholder.

set -euo pipefail

REPO="<owner>/llmwiki"
BINARY="llmwiki-cli"
INSTALL_DIR="${LLMWIKI_INSTALL_DIR:-$HOME/.local/bin}"

# --- Detect OS and architecture ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  TARGET_OS="unknown-linux-musl" ;;
  Darwin) TARGET_OS="apple-darwin" ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "Windows detected. Please use Git Bash or WSL, or run install.ps1 (not yet available)."
    exit 1
    ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET_ARCH="x86_64" ;;
  aarch64|arm64) TARGET_ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

# --- Fetch latest release tag ---
echo "Fetching latest release of $BINARY..."
TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')"
if [ -z "$TAG" ]; then
  echo "Could not determine latest release tag." >&2
  exit 1
fi
echo "Latest version: $TAG"

# --- Download ---
ASSET="${BINARY}-${TARGET}.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
SHA_URL="$URL.sha256"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "Downloading $URL..."
curl -fsSL -o "$TMP/$ASSET" "$URL"

echo "Verifying SHA256..."
EXPECTED="$(curl -fsSL "$SHA_URL" | awk '{print $1}')"
ACTUAL="$(sha256sum "$TMP/$ASSET" | awk '{print $1}')"
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "SHA256 mismatch: expected $EXPECTED, got $ACTUAL" >&2
  exit 1
fi

# --- Extract + install ---
tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMP/$BINARY" "$INSTALL_DIR/$BINARY"

# --- Banner ---
echo ""
echo "✓ Installed $BINARY $TAG to $INSTALL_DIR"
echo ""
echo "Next steps:"
echo "  1. Ensure $INSTALL_DIR is in your PATH:"
echo "       export PATH=\"$INSTALL_DIR:\$PATH\""
echo "  2. Verify the install:"
echo "       $BINARY doctor"
echo "  3. If you have the wiki skill installed, restart your AI agent"
echo "     so it picks up the new binary path."