#!/usr/bin/env bash
# M2 desktop gate: automated checks plus explicit manual items.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

"$root/scripts/m3-gate.sh"

cargo test -p orexis-desktop-core
cargo check --manifest-path crates/desktop/ui/Cargo.toml --target wasm32-unknown-unknown

if ! test -f "crates/desktop/src-tauri/icons/icon.png"; then
  echo "missing desktop bundle icon: crates/desktop/src-tauri/icons/icon.png" >&2
  exit 1
fi

if grep -RIn --include='*.rs' -E 'TcpListener|axum::serve|hyper::Server|warp::serve' crates/desktop crates/daemon crates/composition >/dev/null; then
  echo "desktop/daemon must not expose an HTTP server" >&2
  exit 1
fi

for panel in status left transcript composer right settings; do
  if ! test -f "crates/desktop/ui/src/components/$panel.rs" && ! test -f "crates/desktop/ui/src/components/$panel/mod.rs"; then
    echo "missing panel component: $panel" >&2
    exit 1
  fi
done

echo
echo "M2 automated gate passed."
echo "Manual (not verified here):"
echo "  - Tauri shell build: needs webkit2gtk-4.1, gtk+-3.0, trunk, tauri-cli"
echo "  - Evaluation cycle 1: wireframes with 3 developers"
echo "  - Evaluation cycle 2: clickable shell with 5 developers"
echo "  - Usability spec targets: record misses and redesign decisions"
