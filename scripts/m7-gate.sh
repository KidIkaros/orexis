#!/usr/bin/env bash
# M7 gate: jsonrpc, mcp, acp, utcp, a2a — sans-io cores, shells, fixtures.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m6-gate.sh"

echo "== protocol suites"
cargo test -p orexis-adapters-jsonrpc
cargo test -p orexis-adapters-protocol-mcp
cargo test -p orexis-adapters-protocol-acp
cargo test -p orexis-adapters-protocol-utcp
cargo test -p orexis-adapters-protocol-a2a

echo "== conformance fixture inventory"
# Every protocol crate carries recorded-transcript fixtures; none requires
# a live peer (the real-peer checks are manual, listed below). Each crate
# is checked against the fixture style it actually uses: inline wire
# strings, a scripted mock server, or scripted transports.
check_fixtures() {
  local crate="$1" pattern="$2"
  if ! grep -RIn --include='*.rs' -qE "$pattern" "crates/adapters-$crate"; then
    echo "no recorded-transcript fixtures in adapters-$crate" >&2
    exit 1
  fi
}
check_fixtures jsonrpc        '"jsonrpc"'
check_fixtures protocol-mcp   'mock_mcp_server|MockTransport'
check_fixtures protocol-acp   '"jsonrpc"'
check_fixtures protocol-utcp  'MockTransport|CommandRunner|PeerFactory'
check_fixtures protocol-a2a   'MockTransport'
echo "every protocol crate has recorded-transcript fixtures"

echo
echo "M7 automated gate passed."
echo "Manual (not verified here): one real peer per protocol:"
echo "  - a real stdio MCP server driven end-to-end"
echo "  - an ACP client driving 'orexis acp'"
echo "  - a real HTTP UTCP manual called end-to-end"
echo "  - a real A2A sample agent completing a task"
