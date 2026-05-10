#!/usr/bin/env bash
# Build jot for Linux x86_64 (musl, static binary)
set -euo pipefail

TARGET="x86_64-unknown-linux-musl"
BIN_NAME="jot-linux-x86_64"
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
cp "target/$TARGET/release/jot" "$DIST/$BIN_NAME"
echo ">> Output: dist/$BIN_NAME"
