#!/usr/bin/env bash
# Build jot for Linux ARM64 (musl, static binary)
# Requires: cargo-zigbuild, zig (pip install ziglang), rustup target
set -euo pipefail

TARGET="aarch64-unknown-linux-musl"
BIN_NAME="jot-linux-aarch64"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"

# ── Prerequisites ─────────────────────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
  echo "error: cargo not found — install Rust from https://rustup.rs" >&2; exit 1
fi
if ! command -v cargo-zigbuild &>/dev/null; then
  echo "error: cargo-zigbuild not found — run: cargo install cargo-zigbuild" >&2; exit 1
fi
if ! python3 -c "import ziglang" &>/dev/null && ! command -v zig &>/dev/null; then
  echo "error: zig not found — run: pip install ziglang  or  brew install zig" >&2; exit 1
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
cargo zigbuild --release -p cli --target "$TARGET"

mkdir -p "$DIST"
cp "target/$TARGET/release/jot" "$DIST/$BIN_NAME"
echo ">> Output: dist/$BIN_NAME"
