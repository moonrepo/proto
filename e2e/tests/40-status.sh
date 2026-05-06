#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

out=$(proto status 2>&1)

# At minimum the node install (10-install-node) must show up
assert_contains "$out" "node"
