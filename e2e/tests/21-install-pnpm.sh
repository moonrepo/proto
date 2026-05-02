#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install pnpm 9

bin=$(proto bin pnpm)
assert_executable "$bin"
