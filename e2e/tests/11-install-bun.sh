#!/usr/bin/env bash
# group: tools
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install bun 1

bin=$(proto bin bun)
assert_executable "$bin"
