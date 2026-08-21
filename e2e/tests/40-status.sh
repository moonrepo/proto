#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

run_probe proto status
echo_probe
assert_probe_ok

# At minimum the node install (10-install-node) must show up
assert_contains "$RUN_ALL" "node"
