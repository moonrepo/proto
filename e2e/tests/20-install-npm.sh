#!/usr/bin/env bash
# requires: 10-install-node
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install npm 10

bin=$(proto bin npm)
assert_executable "$bin"
