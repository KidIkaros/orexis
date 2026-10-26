#!/usr/bin/env bash
# The automated M1 gate. Manual checks are printed at the end.
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
echo "M1 automated gate passed."
echo "Manual: three live providers and one real repository task."
echo "  - OpenAI-compatible: OREXIS_PROFILE=openai OREXIS_MODEL=... OREXIS_FAKE= serve"
echo "  - Anthropic:         OREXIS_PROFILE=anthropic OREXIS_MODEL=... serve"
echo "  - llama.cpp:         OREXIS_PROFILE=llamacpp OREXIS_MODEL=local serve"
echo "  - Real task needs the M3 tool surface (read/write/shell)."
