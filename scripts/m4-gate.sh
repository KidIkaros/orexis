#!/usr/bin/env bash
# M4 gate: recovery, audit, quarantine, plus the automated baseline.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m3-gate.sh"

cargo test -p orexis-daemon --test restart kill_mid_turn_recovers
cargo test -p orexis-daemon --test attach audit_contains
cargo test -p orexis-adapters-skills

echo
echo "M4 automated gate passed."
echo "Manual (not verified here):"
echo "  - SIGKILL a real repository turn, restart, and reattach"
echo "  - Skill signatures (hash integrity is verified; signing is later)"
