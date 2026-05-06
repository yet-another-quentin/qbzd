#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/src-tauri/src/commands_v2"

if [[ ! -d "$TARGET_DIR" ]]; then
  echo "ERROR: Directory not found: $TARGET_DIR" >&2
  exit 2
fi

PATTERN='crate::commands::|crate::library::commands::|crate::cast::[^[:space:]]*::commands::|crate::offline_cache::commands::'

# Use grep instead of rg so the check works in environments without ripgrep.
# grep -r exits 0 on match, 1 on no-match, 2 on error. Suppress the "no-match"
# exit by using `|| true` and inspecting captured output.
matches=$(grep -rEn --include='*.rs' "$PATTERN" "$TARGET_DIR" || true)
if [[ -n "$matches" ]]; then
  echo "$matches"
  echo
  echo "FAIL: Forbidden legacy delegation found in commands_v2/"
  exit 1
fi

echo "OK: commands_v2/ backend purity check passed"
