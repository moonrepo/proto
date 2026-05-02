#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install go 1.23

bin=$(proto bin go)
assert_executable "$bin"
