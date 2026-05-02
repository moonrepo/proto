#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install rust stable

bin=$(proto bin rust)
assert_executable "$bin"
