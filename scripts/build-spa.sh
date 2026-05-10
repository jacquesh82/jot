#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../spa"
bun install --frozen-lockfile
bun run build
