#!/usr/bin/env bash
set -euo pipefail

version_file="static/web-vault/version.json"
vw_version_file="static/web-vault/vw-version.json"

extract_version() {
  sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

web_version="$(extract_version "$version_file")"
vw_version="$(extract_version "$vw_version_file")"

if [[ -z "$web_version" || -z "$vw_version" ]]; then
  echo "Unable to read Web Vault version metadata" >&2
  exit 1
fi

if [[ "$web_version" != "$vw_version" ]]; then
  echo "Web Vault versions differ: version.json=$web_version vw-version.json=$vw_version" >&2
  exit 1
fi

if ! [[ "$web_version" =~ ^[0-9]{4}\.[0-9]+\.[0-9]+$ ]]; then
  echo "Unsupported Web Vault version format: $web_version" >&2
  exit 1
fi

echo "Web Vault version: $web_version"
