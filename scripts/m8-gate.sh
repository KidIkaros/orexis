#!/usr/bin/env bash
# M8 gate: the team domain — board/bus/blackboard adapters, scheduler,
# supervisor, worker turns, the team loop — on top of the M7 chain.
# The benchmark is not run here: it needs a live provider and it is the
# ship/experimental decision, not a regression check (spec §Gate).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m7-gate.sh"

echo "== team adapters"
cargo test -p orexis-adapters-board-sqlite
cargo test -p orexis-adapters-message-sqlite
cargo test -p orexis-adapters-artifacts-fs

echo "== team use cases"
cargo test -p orexis-usecases

echo "== worker turns and the team loop"
cargo test -p orexis-daemon --test worker
cargo test -p orexis-daemon --test team_loop

echo "== bench"
cargo test -p orexis-bench

echo "== team mode is opt-in (invariant 19)"
# The team loop and the board tools exist only behind the flag, and the
# flag is read by the daemon, not constructed inside the domain.
grep -q 'pub team: bool' crates/daemon/src/server.rs
grep -q 'OREXIS_TEAM' crates/daemon/src/env.rs
if grep -RIn --include='*.rs' -q 'OREXIS_TEAM' crates/composition crates/usecases crates/ports; then
  echo "team mode must be read at the daemon boundary, not inward of it" >&2
  exit 1
fi
echo "team mode is behind OREXIS_TEAM"

echo
echo "M8 automated gate passed."
echo "Benchmark (the ship/experimental decision), not run here:"
echo "  orexis bench run --tasks bench/tasks/breadth.yaml \\"
echo "    --profile P --model M --modes serial --out /tmp/serial.jsonl"
echo "  orexis bench run --tasks bench/tasks/breadth.yaml \\"
echo "    --profile P --model M --modes team   --out /tmp/team.jsonl"
echo "  orexis bench compare --serial /tmp/serial.jsonl --team /tmp/team.jsonl"
echo "Manual (not verified here): one real team run on a real repository,"
echo "with the fleet panel showing live workers."