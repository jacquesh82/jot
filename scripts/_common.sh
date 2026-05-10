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

# Common locations that may not be in PATH yet
export PATH="$HOME/.local/bin:$HOME/.local/share/zig:$PATH"

if ! command -v zig &>/dev/null; then

  # Strategy 1: pipx — resolves the actual bin dir dynamically
  if command -v pipx &>/dev/null; then
    echo ">> Installing zig via pipx (ziglang)"
    pipx install ziglang 2>/dev/null || pipx upgrade ziglang 2>/dev/null || true

    PIPX_BIN="$(pipx environment --value PIPX_BIN_DIR 2>/dev/null || echo "$HOME/.local/bin")"
    export PATH="$PIPX_BIN:$PATH"
  fi

  # Strategy 2: direct download from ziglang.org (no Python required)
  if ! command -v zig &>/dev/null; then
    ZIG_VERSION="0.14.0"
    ZIG_DIR="$HOME/.local/share/zig"

    case "$(uname -s)" in
      Linux)  ZIG_OS="linux"  ;;
      Darwin) ZIG_OS="macos"  ;;
      *)      echo "error: unsupported OS $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
      x86_64)         ZIG_ARCH="x86_64"  ;;
      aarch64|arm64)  ZIG_ARCH="aarch64" ;;
      *)              echo "error: unsupported arch $(uname -m)" >&2; exit 1 ;;
    esac

    ZIG_TARBALL="zig-${ZIG_OS}-${ZIG_ARCH}-${ZIG_VERSION}.tar.xz"
    ZIG_URL="https://ziglang.org/download/${ZIG_VERSION}/${ZIG_TARBALL}"

    if [[ ! -x "$ZIG_DIR/zig" ]]; then
      echo ">> Downloading zig ${ZIG_VERSION} (${ZIG_OS}-${ZIG_ARCH})"
      mkdir -p "$ZIG_DIR"
      if command -v curl &>/dev/null; then
        curl -fsSL "$ZIG_URL" | tar -xJ -C "$ZIG_DIR" --strip-components=1
      elif command -v wget &>/dev/null; then
        wget -qO- "$ZIG_URL" | tar -xJ -C "$ZIG_DIR" --strip-components=1
      else
        echo "error: curl and wget not found — cannot download zig" >&2; exit 1
      fi
    fi

    export PATH="$ZIG_DIR:$PATH"
  fi

  if ! command -v zig &>/dev/null; then
    echo "error: zig not found after all install attempts." >&2
    echo "       Try manually: pacman -S zig | brew install zig | https://ziglang.org/download/" >&2
    exit 1
  fi
fi
