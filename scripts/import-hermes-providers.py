#!/usr/bin/env python3
"""Extract Hermes provider data (MIT) into a data-only snapshot.

Runs Hermes's own discovery in an isolated HERMES_HOME. No Hermes code is
vendored; only name/base_url/api_mode/auth/env/aliases/vision are recorded.
"""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
hermes = Path(os.environ.get("HERMES_AGENT", "/home/ikaaros/.hermes/hermes-agent"))
sys.path.insert(0, str(hermes))
os.environ.setdefault("HERMES_HOME", "/tmp/hermes-provider-extract")

from providers import list_providers  # noqa: E402

commit = subprocess.check_output(
    ["git", "-C", str(hermes), "rev-parse", "--short", "HEAD"], text=True
).strip()

providers = []
for profile in list_providers():
    providers.append(
        {
            "name": profile.name,
            "display_name": getattr(profile, "display_name", None),
            "base_url": getattr(profile, "base_url", "") or "",
            "api_mode": getattr(profile, "api_mode", "") or "",
            "auth_type": getattr(profile, "auth_type", "") or "",
            "env_vars": list(getattr(profile, "env_vars", ()) or ()),
            "aliases": list(getattr(profile, "aliases", ()) or ()),
            "supports_vision": bool(getattr(profile, "supports_vision", False)),
            "hidden": bool(getattr(profile, "hidden", False)),
        }
    )

snapshot = {
    "source": f"hermes-agent@{commit}",
    "license": "MIT",
    "fetched": "2026-10-26",
    "providers": providers,
}

out = repo / "crates/adapters-provider-catalog/providers/hermes-providers.json"
out.write_text(json.dumps(snapshot, indent=2) + "\n")
license_src = hermes / "LICENSE"
if license_src.exists():
    shutil.copy(license_src, out.parent / "hermes.LICENSE")
print(f"wrote {len(providers)} providers from {commit} -> {out}")
