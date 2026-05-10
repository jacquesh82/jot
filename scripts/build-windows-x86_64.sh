#!/usr/bin/env bash
# Build jot for Windows x86_64 (static, cross-compiled via zigbuild)
# Can run on Linux or macOS.
set -euo pipefail

TARGET="x86_64-pc-windows-gnu"
BIN_NAME="jot-windows-x86_64.exe"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"

# ── Prerequisites (auto-install cargo-zigbuild + zig) ─────────────────────────

# shellcheck source=_common.sh
source "$ROOT/scripts/_common.sh"

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
cargo zigbuild --release -p cli --target "$TARGET"

mkdir -p "$DIST"
cp "target/$TARGET/release/jot.exe" "$DIST/$BIN_NAME"
echo ">> Output: dist/$BIN_NAME"
