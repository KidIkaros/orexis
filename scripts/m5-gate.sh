#!/usr/bin/env bash
# M5 gate: entity persistence, drives, affect, memory, identity, plus M4.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m4-gate.sh"

cargo test -p orexis-core entity
cargo test -p orexis-usecases entity
cargo test -p orexis-usecases memory
cargo test -p orexis-adapters-entity-sqlite
cargo test -p orexis-daemon --test attach identity
cargo test -p orexis-adapters-tui self_view

if grep -RIn --include='*.rs' -E 'embedding|vector_store|faiss|qdrant' \
    crates/core/src/entity crates/usecases/src/memory.rs; then
  echo "entity memory must stay lexical; no embeddings" >&2
  exit 1
fi

echo
echo "M5 automated gate passed."
echo "Manual (not verified here):"
echo "  - /self against a real repository"
echo "  - entity persistence across a live model switch"
