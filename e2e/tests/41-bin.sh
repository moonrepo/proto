#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Walk every built-in tool we try to install. Tools whose install test failed
# or was skipped will return non-zero from `proto bin` and are silently
# skipped here — failures already surfaced upstream.
checked=0
for tool in node bun deno go python ruby rust uv moon npm pnpm yarn poetry; do
  if bin=$(proto bin "$tool" 2>/dev/null); then
    assert_executable "$bin"
    checked=$((checked + 1))
  fi
done

# Sanity: at least one tool should resolve (node, given dep ordering)
[[ $checked -gt 0 ]] || fail "no tools had a resolvable bin path"
echo "verified bin path for $checked tool(s)"
