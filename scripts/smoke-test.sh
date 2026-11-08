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

admin_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' "$base_url/admin")"
[[ "$admin_status" == "401" || "$admin_status" == "200" ]] || {
  echo "/admin is not served by the Worker (got $admin_status; 405 means it falls through to static assets)" >&2
  exit 1
}

prefixed_status="$(curl --silent --show-error --output /dev/null --write-out '%{http_code}' \
  --request POST --header 'Content-Type: application/json' --data '{}' \
  "$base_url/admin/identity/accounts/prelogin")"
[[ "$prefixed_status" != "405" ]] || {
  echo "POST /admin/identity/... is not reaching the Worker (405 from static assets)" >&2
  exit 1
}

echo "Smoke test passed for $base_url"
