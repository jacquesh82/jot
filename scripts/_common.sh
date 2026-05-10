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

# ── zig ───────────────────────────────────────────────────────────────────────

if ! command -v zig &>/dev/null; then
  # ~/.local/bin may hold pipx-installed binaries
  export PATH="$HOME/.local/bin:$PATH"
fi

if ! command -v zig &>/dev/null; then
  # Strategy 1: pipx (recommended on Arch Linux and other PEP 668 systems)
  if command -v pipx &>/dev/null; then
    echo ">> Installing zig via pipx (ziglang)"
    pipx install ziglang --quiet 2>/dev/null \
      || pipx upgrade ziglang --quiet 2>/dev/null \
      || true
    export PATH="$HOME/.local/bin:$PATH"

  # Strategy 2: pip with --break-system-packages fallback
  elif command -v pip3 &>/dev/null || command -v pip &>/dev/null; then
    PIP=$(command -v pip3 2>/dev/null || command -v pip)
    if ! python3 -c "import ziglang" &>/dev/null; then
      echo ">> Installing ziglang via $PIP"
      "$PIP" install --quiet ziglang 2>/dev/null \
        || "$PIP" install --quiet --break-system-packages ziglang
    fi
    # ziglang ships its own zig binary — add it to PATH
    ZIG_BIN="$(python3 -c \
      "import ziglang, pathlib; print(pathlib.Path(ziglang.__file__).parent / 'zig')" \
      2>/dev/null || true)"
    [[ -n "$ZIG_BIN" && -x "$ZIG_BIN" ]] && export PATH="$(dirname "$ZIG_BIN"):$PATH"

  else
    echo "error: cannot install zig — pipx and pip not found." >&2
    echo "       Install zig manually: https://ziglang.org/download/" >&2
    exit 1
  fi

  if ! command -v zig &>/dev/null; then
    echo "error: zig still not found after installation." >&2
    echo "       Try: pacman -S zig  |  brew install zig  |  https://ziglang.org/download/" >&2
    exit 1
  fi
fi
