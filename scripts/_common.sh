#!/usr/bin/env bash
# Sourced by cross-compilation build scripts — do not execute directly.
# Ensures cargo, cargo-zigbuild, and zig are available, installing them if needed.

# ── cargo / rustup ────────────────────────────────────────────────────────────

# ~/.cargo/bin may not be in PATH in non-interactive shells
export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v cargo &>/dev/null; then
  echo "error: cargo not found — install Rust from https://rustup.rs" >&2; exit 1
fi

# ── cargo-zigbuild ────────────────────────────────────────────────────────────

if ! command -v cargo-zigbuild &>/dev/null; then
  echo ">> Installing cargo-zigbuild"
  cargo install cargo-zigbuild
fi

# ── zig (via ziglang Python package) ─────────────────────────────────────────

if ! command -v zig &>/dev/null; then
  if command -v pip3 &>/dev/null; then
    PIP=pip3
  elif command -v pip &>/dev/null; then
    PIP=pip
  else
    echo "error: zig and pip not found — install zig manually: https://ziglang.org/download/" >&2; exit 1
  fi

  if ! python3 -c "import ziglang" &>/dev/null; then
    echo ">> Installing ziglang via $PIP"
    "$PIP" install --quiet ziglang
  fi

  # ziglang installs a 'zig' wrapper; find it
  ZIG_BIN="$(python3 -c "import ziglang, pathlib; print(pathlib.Path(ziglang.__file__).parent / 'zig')" 2>/dev/null || true)"
  if [[ -n "$ZIG_BIN" && -x "$ZIG_BIN" ]]; then
    export PATH="$(dirname "$ZIG_BIN"):$PATH"
  fi

  if ! command -v zig &>/dev/null; then
    echo "error: zig still not found after installing ziglang — check your Python PATH" >&2; exit 1
  fi
fi
