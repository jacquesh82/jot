#!/usr/bin/env bash
# Sourced by cross-compilation build scripts — do not execute directly.
# Ensures cargo, rustup, cargo-zigbuild, and zig are available.

# ── PATH bootstrap ────────────────────────────────────────────────────────────

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$HOME/.local/share/zig:$PATH"

# ── cargo ─────────────────────────────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
  echo "error: cargo not found — install Rust from https://rustup.rs" >&2; exit 1
fi

# ── rustup ────────────────────────────────────────────────────────────────────
# Arch Linux installs Rust via pacman without rustup; cross-compilation needs it.

if ! command -v rustup &>/dev/null; then
  echo ">> rustup not found — installing via rustup.rs"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path
  export PATH="$HOME/.cargo/bin:$PATH"
  if ! command -v rustup &>/dev/null; then
    echo "error: rustup install failed" >&2; exit 1
  fi
fi

# ── cargo-zigbuild ────────────────────────────────────────────────────────────

if ! command -v cargo-zigbuild &>/dev/null; then
  echo ">> Installing cargo-zigbuild"
  cargo install cargo-zigbuild
fi

# ── zig ───────────────────────────────────────────────────────────────────────

if ! command -v zig &>/dev/null; then

  # Strategy 1: pipx — check bin dir AND the venv (ziglang ≥ 0.16 changed layout)
  if command -v pipx &>/dev/null; then
    echo ">> Installing zig via pipx (ziglang)"
    pipx install ziglang 2>/dev/null || pipx upgrade ziglang 2>/dev/null || true

    # Add the pipx bin dir (older ziglang puts a wrapper here)
    PIPX_BIN="$(pipx environment --value PIPX_BIN_DIR 2>/dev/null || echo "$HOME/.local/bin")"
    export PATH="$PIPX_BIN:$PATH"

    # ziglang ≥ 0.16 dropped the console_scripts entry; find the binary in the venv
    if ! command -v zig &>/dev/null; then
      PIPX_HOME="$(pipx environment --value PIPX_HOME 2>/dev/null \
                   || echo "$HOME/.local/share/pipx")"
      ZIG_IN_VENV="$(find "$PIPX_HOME/venvs/ziglang" \
                       -name 'zig' -type f -executable \
                       ! -name '*.py' ! -name '*.pyc' \
                     2>/dev/null | head -1)"
      [[ -n "$ZIG_IN_VENV" ]] && export PATH="$(dirname "$ZIG_IN_VENV"):$PATH"
    fi
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
      x86_64)        ZIG_ARCH="x86_64"  ;;
      aarch64|arm64) ZIG_ARCH="aarch64" ;;
      *)             echo "error: unsupported arch $(uname -m)" >&2; exit 1 ;;
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
    echo "       Try: pacman -S zig | brew install zig | https://ziglang.org/download/" >&2
    exit 1
  fi
fi

# ── OpenSSL (vendored for musl/Windows cross-compilation) ─────────────────────
# openssl-sys is built with the vendored feature in crates/storage/Cargo.toml;
# perl is required by OpenSSL's configure script during that build.

if ! command -v perl &>/dev/null; then
  echo "error: perl not found — required to build vendored OpenSSL" >&2
  echo "       Install it with: pacman -S perl | brew install perl | apt install perl" >&2
  exit 1
fi

# ── Swagger UI (pre-download for cross-compilation) ───────────────────────────
# utoipa-swagger-ui's build script calls curl at build time; that curl is not
# always reachable in cargo's subprocess environment when cross-compiling.
# Pre-download the zip here (where curl is guaranteed in PATH) and point the
# build script at the local file via SWAGGER_UI_DOWNLOAD_URL.

SWAGGER_UI_VERSION="5.17.12"
SWAGGER_UI_CACHE="$HOME/.local/share/swagger-ui"
SWAGGER_UI_ZIP="$SWAGGER_UI_CACHE/v${SWAGGER_UI_VERSION}.zip"

if [[ ! -f "$SWAGGER_UI_ZIP" ]]; then
  echo ">> Downloading Swagger UI ${SWAGGER_UI_VERSION}"
  mkdir -p "$SWAGGER_UI_CACHE"
  if command -v curl &>/dev/null; then
    curl -fsSL \
      "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v${SWAGGER_UI_VERSION}.zip" \
      -o "$SWAGGER_UI_ZIP" \
      || { echo "error: failed to download Swagger UI" >&2; exit 1; }
  elif command -v wget &>/dev/null; then
    wget -qO "$SWAGGER_UI_ZIP" \
      "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v${SWAGGER_UI_VERSION}.zip" \
      || { echo "error: failed to download Swagger UI" >&2; exit 1; }
  else
    echo "error: curl and wget not found — cannot download Swagger UI" >&2; exit 1
  fi
fi

export SWAGGER_UI_DOWNLOAD_URL="file://$SWAGGER_UI_ZIP"
