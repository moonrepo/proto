#!/usr/bin/env bash
# group: tools
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install moon

bin=$(proto bin moon)
assert_executable "$bin"
