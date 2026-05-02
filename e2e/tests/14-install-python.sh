#!/usr/bin/env bash
# group: tools
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install python 3.12

bin=$(proto bin python)
assert_executable "$bin"
