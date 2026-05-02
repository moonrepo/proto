#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install ruby 3.3

bin=$(proto bin ruby)
assert_executable "$bin"
