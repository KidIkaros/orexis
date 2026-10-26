#!/usr/bin/env bash
# Refresh the vendored models.dev snapshot (MIT).
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
providers="$root/crates/adapters-provider-catalog/providers"
curl -fsSL https://models.dev/api.json -o "$providers/models-dev.json"
curl -fsSL https://raw.githubusercontent.com/sst/models.dev/dev/LICENSE -o "$providers/models-dev.LICENSE"
echo "refreshed $providers/models-dev.json"
