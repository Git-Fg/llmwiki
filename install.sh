#!/usr/bin/env bash
# llmwiki-cli installer (bash).
# Usage:
#   curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | bash
#   curl -LsSf https://github.com/Git-Fg/llmwiki/releases/latest/download/install.sh | bash -s -- --bin-dir /usr/local/bin

set -euo pipefail

REPO="Git-Fg/llmwiki"
BINARY="llmwiki-cli"
VERSION="latest"
BIN_DIR="${LLMWIKI_BIN_DIR:-${LLMWIKI_INSTALL_DIR:-$HOME/.local/bin}}"
VERBOSE=0
FORCE=0

usage() {
  cat <<EOF
Usage: bash install.sh [options]

Options:
  --version <ver>    Release tag to install (default: latest)
  --bin-dir <dir>    Install directory (default: \$HOME/.local/bin)
  --verbose          Print every command as it runs
  --force            Overwrite an existing binary without prompting
  --help             Show this help and exit

Environment overrides:
  LLMWIKI_BIN_DIR    Same as --bin-dir
  LLMWIKI_INSTALL_DIR  Legacy alias for --bin-dir

Examples:
  bash install.sh --version v0.3.7
  bash install.sh --bin-dir /usr/local/bin --force
  curl -LsSf .../install.sh | bash -s -- --verbose
EOF
}

# Parse flags
while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --bin-dir) BIN_DIR="$2"; shift 2 ;;
    --verbose) VERBOSE=1; shift ;;
    --force)   FORCE=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$VERBOSE" = "1" ]; then set -x; fi

# Pick a download tool (curl preferred, wget fallback).
if command -v curl >/dev/null 2>&1; then
  DOWNLOAD() { curl -fsSL "$1"; }
  DOWNLOAD_FILE() { curl -fsSL -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  DOWNLOAD() { wget -qO- "$1"; }
  DOWNLOAD_FILE() { wget -qO "$2" "$1"; }
else
  echo "ERROR: neither curl nor wget is installed." >&2
  exit 1
fi

# --- Detect OS and architecture ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  TARGET_OS="unknown-linux-musl" ;;
  Darwin) TARGET_OS="apple-darwin" ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "Windows detected via uname. Use install.ps1 in PowerShell, or WSL+this script." >&2
    exit 1
    ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET_ARCH="x86_64" ;;
  aarch64|arm64) TARGET_ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

# --- Resolve the download URL ---
if [ "$VERSION" = "latest" ]; then
  BASE="https://github.com/$REPO/releases/latest/download"
else
  BASE="https://github.com/$REPO/releases/download/$VERSION"
fi

ASSET="${BINARY}-${TARGET}.tar.gz"
URL="$BASE/$ASSET"
SHA_URL="$URL.sha256"

TMP="$(mktemp -d 2>/dev/null || mktemp -d -t llmwiki)"
trap 'rm -rf "$TMP"' EXIT INT TERM

# --- Overwrite guard ---
if [ -e "$BIN_DIR/$BINARY" ] && [ "$FORCE" != "1" ]; then
  echo "Existing binary found at $BIN_DIR/$BINARY. Use --force to overwrite." >&2
  exit 1
fi

# --- Download ---
echo "Downloading $URL..."
DOWNLOAD_FILE "$URL" "$TMP/$ASSET"

# --- Verify SHA256 ---
echo "Verifying SHA256..."
EXPECTED="$(DOWNLOAD "$SHA_URL" | awk '{print $1}')"
if [ -z "$EXPECTED" ]; then
  echo "Could not fetch SHA256 from $SHA_URL" >&2
  exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "$TMP/$ASSET" | awk '{print $1}')"
else
  ACTUAL="$(shasum -a 256 "$TMP/$ASSET" | awk '{print $1}')"
fi
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "SHA256 mismatch:" >&2
  echo "  expected: $EXPECTED" >&2
  echo "  actual:   $ACTUAL" >&2
  exit 1
fi

# --- Extract + install ---
tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$BIN_DIR"
install -m 0755 "$TMP/$BINARY" "$BIN_DIR/$BINARY"

# --- Banner ---
TAG="$VERSION"
[ "$TAG" = "latest" ] && TAG="(latest)"
echo ""
echo "✓ Installed $BINARY $TAG to $BIN_DIR"
echo ""
echo "Next steps:"
echo "  1. Ensure $BIN_DIR is in your PATH:"
echo "       export PATH=\"$BIN_DIR:\$PATH\""
echo "  2. Verify the install:"
echo "       $BINARY doctor"
echo "  3. If you have the wiki skill installed, restart your AI agent"
echo "     so it picks up the new binary path."