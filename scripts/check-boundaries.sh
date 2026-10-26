#!/usr/bin/env bash
# Enforce the Dependency Rule (AGENTS.md rules 1-9).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fail=0

check_absent() {
  local pattern="$1"
  local label="$2"
  shift 2
  local hits
  hits="$(grep -rn "$pattern" "$@" --include='*.rs' 2>/dev/null || true)"
  if [ -n "$hits" ]; then
    echo "boundary violation: $label"
    echo "$hits" | head -5
    fail=1
  fi
}

check_absent 'tokio' 'tokio inward of adapters' \
  "$root/crates/core" "$root/crates/ports" "$root/crates/usecases"

check_absent 'orexis_adapters' 'adapter import inward of adapters' \
  "$root/crates/core" "$root/crates/ports" "$root/crates/usecases"

check_absent 'orexis_usecases' 'usecases import in an adapter' \
  "$root/crates/adapters-fake" \
  "$root/crates/adapters-session-sqlite" \
  "$root/crates/adapters-provider-catalog" \
  "$root/crates/adapters-provider-openai" \
  "$root/crates/adapters-provider-anthropic" \
  "$root/crates/adapters-provider-llamacpp" \
  "$root/crates/adapters-codec" \
  "$root/crates/adapters-http" \
  "$root/crates/adapters-context-fs" \
  "$root/crates/adapters-tui"


check_manifest_absent() {
  local pattern="$1"
  local label="$2"
  shift 2
  local hits
  hits="$(grep -n "$pattern" "$@" 2>/dev/null || true)"
  if [ -n "$hits" ]; then
    echo "boundary violation: $label"
    echo "$hits" | head -5
    fail=1
  fi
}

check_manifest_absent 'tokio' 'tokio dependency inward of adapters' \
  "$root/crates/core/Cargo.toml" "$root/crates/ports/Cargo.toml" "$root/crates/usecases/Cargo.toml"

check_manifest_absent 'orexis-adapters' 'adapter dependency inward of adapters' \
  "$root/crates/core/Cargo.toml" "$root/crates/ports/Cargo.toml" "$root/crates/usecases/Cargo.toml"

check_manifest_absent 'orexis-usecases' 'usecases dependency in an adapter' \
  "$root"/crates/adapters-*/Cargo.toml

for manifest in "$root"/crates/adapters-provider-{openai,anthropic,llamacpp}/Cargo.toml; do
  [ -f "$manifest" ] || continue
  own="$(basename "$(dirname "$manifest")")"
  hits="$(grep 'orexis-adapters-provider-' "$manifest" | grep -v 'orexis-adapters-provider-catalog' | grep -v "$own" || true)"
  if [ -n "$hits" ]; then
    echo "boundary violation: provider-to-provider dependency in $manifest"
    echo "$hits"
    fail=1
  fi
done

if [ -d "$root/crates/utils" ]; then
  echo "boundary violation: shared utils crate"
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi
echo "boundaries ok"
