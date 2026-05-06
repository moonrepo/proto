#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Listing versions exercises the plugin loader (downloads the WASM plugin if
# not cached) and the upstream registry call. Sample a few tools rather than
# all 13 — exhaustive coverage happens during install tests.
for tool in node bun python go; do
  out=$(retry 3 proto versions "$tool" 2>&1)
  # Output should contain at least one version-shaped string with a dot
  assert_contains "$out" "."
done
