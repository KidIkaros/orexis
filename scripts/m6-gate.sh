#!/usr/bin/env bash
# M6 gate: corridor, quarantine, validation, tiers, rollback, 100 cycles.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m5-gate.sh"

cargo test -p orexis-core evolution
cargo test -p orexis-usecases evolution
cargo test -p orexis-usecases apply
cargo test -p orexis-adapters-evolution-git
cargo test -p orexis-composition --test evolution_apply
cargo test -p orexis-composition --test evolution_100_cycles

echo
echo "M6 automated gate passed."
echo "Manual (not verified here):"
echo "  - inspect /evolve and the ledger on a real repository"
echo "  - evolution calibration after 100 cycles"
