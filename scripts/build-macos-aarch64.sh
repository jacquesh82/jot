#!/usr/bin/env bash
# Build jot for macOS Apple Silicon (aarch64, dynamic linking)
# Must run on a macOS host.
# Requires: rustup target
set -euo pipefail

TARGET="aarch64-apple-darwin"
BIN_NAME="jot-macos-aarch64"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"

# ── Prerequisites ─────────────────────────────────────────────────────────────

if [[ "$(uname)" != "Darwin" ]]; then
  echo "error: macOS cross-compilation is not supported — run this script on a Mac" >&2; exit 1
fi
if ! command -v cargo &>/dev/null; then
  echo "error: cargo not found — install Rust from https://rustup.rs" >&2; exit 1
fi

# ── Rust target ───────────────────────────────────────────────────────────────

if ! rustup target list --installed | grep -q "$TARGET"; then
  echo ">> Adding Rust target $TARGET"
  rustup target add "$TARGET"
fi

# ── SPA ───────────────────────────────────────────────────────────────────────

echo ">> Building SPA"
bash "$ROOT/scripts/build-spa.sh"

# ── Binary ────────────────────────────────────────────────────────────────────

echo ">> Building $BIN_NAME"
cd "$ROOT"
cargo build --release -p cli --target "$TARGET"

mkdir -p "$DIST"
cp "target/$TARGET/release/jot" "$DIST/$BIN_NAME"
echo ">> Output: dist/$BIN_NAME"
