#!/usr/bin/env bash
set -euo pipefail

base_url="${1:?usage: smoke-test.sh https://worker.example.com}"
base_url="${base_url%/}"

config="$(curl --fail --silent --show-error "$base_url/api/config")"
version="$(curl --fail --silent --show-error "$base_url/api/version")"
alive="$(curl --fail --silent --show-error "$base_url/api/alive")"

printf '%s' "$config" | grep -Eq '"object"[[:space:]]*:[[:space:]]*"config"' || {
  echo "config response is not a Bitwarden config object" >&2
  exit 1
}

printf '%s' "$config" | grep -q '"identity"' || {
  echo "config response does not include identity endpoint" >&2
  exit 1
}

[[ "$version" == '"'*'"' ]] || {
  echo "version response is not a JSON string" >&2
  exit 1
}

[[ -n "$alive" ]] || {
  echo "alive response is empty" >&2
  exit 1
}

echo "Smoke test passed for $base_url"
