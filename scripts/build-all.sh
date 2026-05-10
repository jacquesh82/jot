#!/usr/bin/env bash
# Build jot for all supported platforms.
# Linux and Windows binaries can be cross-compiled on any host via zigbuild.
# macOS binaries require a macOS host and are skipped on other platforms.
#
# Usage:
#   ./scripts/build-all.sh            # build all applicable targets
#   ./scripts/build-all.sh --no-spa   # skip SPA rebuild (uses existing dist/)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
SCRIPTS="$ROOT/scripts"
NO_SPA=false

for arg in "$@"; do
  [[ "$arg" == "--no-spa" ]] && NO_SPA=true
done

# Build the SPA once upfront unless skipped
if [[ "$NO_SPA" == false ]]; then
  echo "════════════════════════════════════════"
  echo " Building SPA"
  echo "════════════════════════════════════════"
  bash "$SCRIPTS/build-spa.sh"
  # Subsequent scripts will skip SPA since dist/ already has the assets.
  # We monkey-patch by passing a no-op build-spa for the sub-scripts via env,
  # but the simpler approach is just to let them re-run it — it's idempotent.
fi

# ── Cross-compiled targets (Linux + Windows, any host) ────────────────────────

CROSS_TARGETS=(
  "build-linux-x86_64.sh"
  "build-linux-aarch64.sh"
  "build-windows-x86_64.sh"
)

for script in "${CROSS_TARGETS[@]}"; do
  echo ""
  echo "════════════════════════════════════════"
  echo " $script"
  echo "════════════════════════════════════════"
  bash "$SCRIPTS/$script"
done

# ── macOS targets (only on macOS) ────────────────────────────────────────────

MACOS_TARGETS=(
  "build-macos-x86_64.sh"
  "build-macos-aarch64.sh"
)

if [[ "$(uname)" == "Darwin" ]]; then
  for script in "${MACOS_TARGETS[@]}"; do
    echo ""
    echo "════════════════════════════════════════"
    echo " $script"
    echo "════════════════════════════════════════"
    bash "$SCRIPTS/$script"
  done
else
  echo ""
  echo ">> Skipping macOS targets (not on Darwin)"
fi

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════"
echo " Build complete — dist/"
echo "════════════════════════════════════════"
ls -lh "$DIST"/jot-* 2>/dev/null || echo "(no binaries found in dist/)"
