#!/usr/bin/env bash
# The Orexis installer (ADR-0019, ticket 020): detect the platform,
# download the release artifact, verify it, install to ~/.local/bin.
#
#   curl -fsSL <installer-url> | bash
#
# Verification (the supply-chain floor: verify before execute):
#   - sha256 against the signed manifest, always;
#   - minisign against the pinned public key, when minisign/rsign is
#     available — pin MINISIGN_PUB below when the keypair is generated;
#     while it is empty, a loud warning is printed and only the checksum
#     (TLS + GitHub + manifest) protects the install.
#
# Env:
#   OREXIS_RELEASE_REPO  owner/repo (required until a default is baked in)
#   OREXIS_VERSION       install a specific version instead of latest
#   OREXIS_INSTALL_DIR   where to install (default ~/.local/bin)
#
# Flags:
#   --from DIR   install from a local artifact directory (the manifest,
#                archives, checksums, and signatures) instead of GitHub —
#                offline installs, and how this script is tested.
set -euo pipefail

APP=orexis
# The minisign public key, pinned once the keypair is generated (the
# wizard step in the ticket). Empty means "not pinned yet".
MINISIGN_PUB="untrusted comment: minisign public key: 6A23EAF0ABE697AB
RWSrl+ar8OojapVJ+qEQMRz87e1bCn3a0khKWx1ezLfo8ubx8LKZplBq"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'; NC='\033[0m'
say()  { printf '%s\n' "$1"; }
good() { printf "${GREEN}%s${NC}\n" "$1"; }
warn() { printf "${YELLOW}%s${NC}\n" "$1" >&2; }
bad()  { printf "${RED}%s${NC}\n" "$1" >&2; exit 1; }

from_dir=""
while [ $# -gt 0 ]; do
  case "$1" in
    --from)
      [ $# -ge 2 ] || bad "--from needs a directory"
      from_dir="$2"
      shift 2
      ;;
    *) bad "unknown argument: $1" ;;
  esac
done

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
[ "$os" = "darwin" ] && os="macos"
arch="$(uname -m)"
case "$arch" in
  x86_64) arch="x86_64" ;;
  aarch64 | arm64) arch="aarch64" ;;
  *) bad "unsupported architecture: $arch" ;;
esac
case "$os" in
  linux) target="$arch-unknown-linux-gnu" ;;
  macos) target="$arch-apple-darwin" ;;
  *) bad "unsupported os: $os (Linux and macOS only)" ;;
esac

command -v tar >/dev/null 2>&1 || bad "tar is required"
command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || bad "sha256sum or shasum is required"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "$from_dir" ]; then
  [ -d "$from_dir" ] || bad "--from directory not found: $from_dir"
  fetch() { cp "$from_dir/$1" "$work/$2"; }
else
  REPO="${OREXIS_RELEASE_REPO:-KidIkaros/orexis}"

  command -v curl >/dev/null 2>&1 || bad "curl is required"

  base="https://github.com/$REPO/releases"
  if [ -n "${OREXIS_VERSION:-}" ]; then
    version="$OREXIS_VERSION"
    release_url="$base/download/v$version"
  else
    release_url="$base/latest/download"
    version="$(curl -fsSL "$base/latest" | sed -n 's|.*/tag/v\([^"]*\).*|\1|p' | head -1)"
    [ -n "$version" ] || bad "could not determine the latest version"
  fi
  fetch() { curl -fsSL "$release_url/$1" -o "$work/$2"; }
fi

say "fetching the manifest"
fetch latest.json latest.json
fetch latest.json.minisig latest.json.minisig 2>/dev/null || true

# Verify the manifest's minisign signature when a verifier and a pinned
# key exist; the manifest carries the artifact checksums, so its
# signature is what makes the checksums trustworthy.
verify_minisign() {
  local file="$1"
  [ -n "$MINISIGN_PUB" ] || return 1
  printf '%s\n' "$MINISIGN_PUB" > "$work/pub.key"
  if command -v minisign >/dev/null 2>&1; then
    minisign -V -p "$work/pub.key" -m "$file" || bad "minisign verification failed for $file"
    return 0
  fi
  if command -v rsign >/dev/null 2>&1; then
    rsign verify -p "$work/pub.key" -x "$file.minisig" "$file" || bad "minisign verification failed for $file"
    return 0
  fi
  return 1
}

if verify_minisign "$work/latest.json"; then
  good "manifest signature verified"
else
  if [ -n "$MINISIGN_PUB" ]; then
    bad "minisign is required to verify the pinned key; install it (pkg_add minisign / brew install minisign) and retry"
  else
    warn "WARNING: no minisign public key is pinned in this installer; the checksum below is not signature-backed. Pin MINISIGN_PUB in scripts/install.sh."
  fi
fi

# The manifest is the source of truth for the version and the checksums.
read_manifest() {
  python3 - "$work/latest.json" "$target" "$1" <<'EOF'
import json, sys
manifest = json.load(open(sys.argv[1]))
field = sys.argv[3]
if field == "version":
    print(manifest["version"])
    sys.exit()
target = sys.argv[2]
for artifact in manifest["artifacts"]:
    if artifact["target"] == target:
        print(artifact[field])
        break
else:
    sys.exit(f"no artifact for {target} in the manifest")
EOF
}
version="${OREXIS_VERSION:-$(read_manifest version)}"
artifact="orexis-$version-$target.tar.gz"
say "downloading $artifact"
fetch "$artifact" "$artifact"
fetch "$artifact.minisig" "$artifact.minisig" 2>/dev/null || true

expected="$(read_manifest sha256)"
[ -n "$expected" ] || bad "no artifact for $target in the manifest"

actual="$(sha256sum "$work/$artifact" 2>/dev/null | awk '{print $1}' || shasum -a 256 "$work/$artifact" | awk '{print $1}')"
[ "$actual" = "$expected" ] || bad "checksum mismatch: expected $expected, got $actual"
good "checksum verified"

if [ -n "$MINISIGN_PUB" ] && [ -f "$work/$artifact.minisig" ]; then
  if command -v minisign >/dev/null 2>&1; then
    minisign -V -p "$work/pub.key" -m "$work/$artifact" || bad "minisign verification failed for $artifact"
    good "artifact signature verified"
  elif command -v rsign >/dev/null 2>&1; then
    rsign verify -p "$work/pub.key" -x "$work/$artifact.minisig" "$work/$artifact" || bad "minisign verification failed for $artifact"
    good "artifact signature verified"
  fi
fi

install_dir="${OREXIS_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$install_dir"
tar -xzf "$work/$artifact" -C "$work"
mv "$work/orexis" "$install_dir/orexis"
chmod +x "$install_dir/orexis"
good "installed $APP v$version to $install_dir/orexis"

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    warn "note: $install_dir is not on your PATH; add it to your shell profile"
    ;;
esac

say ""
say "next: run '$APP init' to select a provider (there is no silent default)"
