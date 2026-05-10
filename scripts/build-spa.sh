#!/usr/bin/env bash
set -euo pipefail

# Add common bun install locations to PATH if not already present
export PATH="$HOME/.bun/bin:$PATH"

if ! command -v bun &>/dev/null; then
  echo "error: bun not found — install from https://bun.sh" >&2
  exit 1
fi

cd "$(dirname "$0")/../spa"
bun install --frozen-lockfile
bun run build
