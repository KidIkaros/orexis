#!/usr/bin/env bash
# The release pipeline (ADR-0019, ticket 020): build a target, archive it
# with its checksum and minisign signature; assemble the signed manifest.
#
# Usage:
#   scripts/release.sh archive TARGET [OUTDIR]   one target: build, tar.gz,
#                                                sha256, minisign sign
#   scripts/release.sh manifest ARTDIR OUTDIR    latest.json from the
#                                                artifacts, then sign it
#   scripts/release.sh dry-run [OUTDIR]          archive the host target,
#                                                no signing
#
# Env:
#   OREXIS_SIGNING_KEY   path to the minisign private key ( signing modes )
#   OREXIS_RELEASE_REPO  owner/repo for the manifest's download URLs
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
caller="$(pwd)"
cd "$root"

# Resolve a caller-relative path to an absolute one; the script changes
# directory to the repo root before doing any work.
abs() {
  case "$1" in
    /*) echo "$1" ;;
    *) echo "$caller/$1" ;;
  esac
}

version="$(grep -m1 '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')"

die() {
  echo "release: $*" >&2
  exit 1
}

sign() {
  local file="$1"
  local key="${OREXIS_SIGNING_KEY:-}"
  [ -n "$key" ] || die "OREXIS_SIGNING_KEY is not set; signing is required outside --dry-run"
  [ -f "$key" ] || die "signing key not found: $key"
  # minisign or its Rust reimplementation rsign — the formats are the
  # same; the CLIs differ.
  local signer=""
  if command -v minisign >/dev/null 2>&1; then
    signer="minisign"
  elif command -v rsign >/dev/null 2>&1; then
    signer="rsign"
  else
    die "neither minisign nor rsign is installed"
  fi
  rm -f "$file.minisig"
  case "$signer" in
    minisign) minisign -S -s "$key" -m "$file" ;;
    rsign) rsign sign -s "$key" -W "$file" ;;
  esac
}

archive_target() {
  local target="$1"
  local outdir="$2"
  local sign_it="${3:-yes}"
  echo "== build $target"
  # CARGO=cross for cross-compilation (aarch64 linux on an x86_64 host).
  "${CARGO:-cargo}" build --release --target "$target" -p orexis
  local binary="target/$target/release/orexis"
  [ -f "$binary" ] || die "binary not found: $binary"

  echo "== archive"
  local name="orexis-$version-$target"
  local stage
  stage="$(mktemp -d)"
  cp "$binary" "$stage/orexis"
  for doc in README.md LICENSE NOTICE; do
    [ -f "$doc" ] && cp "$doc" "$stage/"
  done
  mkdir -p "$outdir"
  tar -czf "$outdir/$name.tar.gz" -C "$stage" .
  rm -rf "$stage"

  echo "== checksum"
  (
    cd "$outdir"
    if command -v sha256sum >/dev/null 2>&1; then
      sha256sum "$name.tar.gz" > "$name.tar.gz.sha256"
    else
      shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256"
    fi
  )

  if [ "$sign_it" = "yes" ]; then
    echo "== sign"
    sign "$outdir/$name.tar.gz"
  fi
}

build_manifest() {
  local artdir="$1"
  local outdir="$2"
  local repo="${OREXIS_RELEASE_REPO:-}"
  [ -n "$repo" ] || die "OREXIS_RELEASE_REPO is not set (owner/repo)"

  echo "== manifest"
  mkdir -p "$outdir"
  local manifest="$outdir/latest.json"
  {
    echo "{"
    echo "  \"version\": \"$version\","
    echo "  \"artifacts\": ["
    local first=1
    for checksum in "$artdir"/*.tar.gz.sha256; do
      [ -f "$checksum" ] || die "no artifact checksums in $artdir"
      local name hash
      name="$(basename "$checksum" .sha256)"
      hash="$(awk '{print $1}' "$checksum")"
      [ -f "$artdir/$name.minisig" ] || die "unsigned artifact: $name"
      if [ $first -eq 0 ]; then
        echo ","
      fi
      first=0
      printf '    {"target": "%s", "url": "https://github.com/%s/releases/download/v%s/%s", "sha256": "%s"}' \
        "$(target_of "$name")" "$repo" "$version" "$name" "$hash"
    done
    echo "  ]"
    echo "}"
  } > "$manifest"

  echo "== sign manifest"
  sign "$manifest"
}

target_of() {
  # orexis-<version>-<target>.tar.gz -> <target>
  local name="$1"
  local without_version="${name#orexis-}"
  without_version="${without_version#*-}"
  echo "${without_version%.tar.gz}"
}

host_target() {
  rustc -vV | awk '/^host:/ {print $2}'
}

case "${1:-}" in
  archive)
    shift
    [ $# -ge 1 ] || die "archive needs a target"
    target="$1"
    outdir="$(abs "${2:-dist}")"
    mkdir -p "$outdir"
    archive_target "$target" "$outdir"
    ;;
  manifest)
    shift
    [ $# -ge 2 ] || die "manifest needs ARTDIR and OUTDIR"
    artdir="$(abs "$1")"
    outdir="$(abs "$2")"
    mkdir -p "$outdir"
    build_manifest "$artdir" "$outdir"
    ;;
  dry-run)
    # No signing: the archive and checksum prove the pipeline; the
    # signature step is exercised when a key exists.
    outdir="$(abs "${2:-dist-dry}")"
    rm -rf "$outdir"
    mkdir -p "$outdir"
    archive_target "$(host_target)" "$outdir" no
    echo "dry-run complete: $outdir (unsigned)"
    ;;
  *)
  die "usage: scripts/release.sh { archive TARGET [OUTDIR] | manifest ARTDIR OUTDIR | dry-run [OUTDIR] }"
  ;;
esac
