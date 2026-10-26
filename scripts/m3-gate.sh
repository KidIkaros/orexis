#!/usr/bin/env bash
# The automated M3 gate. Manual live-provider runs are printed at the end.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

echo "== fmt"
cargo fmt --all -- --check

echo "== clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "== tests"
cargo test --workspace

echo "== boundaries"
scripts/check-boundaries.sh

echo "== seam graph"
scripts/seam-graph.sh > /tmp/orexis-seams.md
diff -u docs/orexis/seams.md /tmp/orexis-seams.md

echo
echo "M3 automated gate passed."
echo "Manual: live providers and a TUI session against a real repository."
echo "  - OREXIS_PROFILE=openai    OREXIS_MODEL=... OREXIS_VERIFY='cargo test' serve"
echo "  - OREXIS_PROFILE=anthropic OREXIS_MODEL=... serve"
echo "  - OREXIS_PROFILE=llamacpp  OREXIS_MODEL=local serve"
echo "  - then: orexis-adapters-tui run --socket ... (or the attach client)"
