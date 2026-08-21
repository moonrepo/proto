#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Listing versions exercises the plugin loader (downloads the WASM plugin if
# not cached) and the upstream registry call. Sample a few tools rather than
# all 13 — exhaustive coverage happens during install tests.
for tool in node bun python go; do
  run_probe retry 3 proto versions "$tool"
  echo_probe
  assert_probe_ok
  # Output should contain at least one version-shaped string with a dot
  assert_contains "$RUN_ALL" "."
done
