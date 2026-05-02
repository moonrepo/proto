#!/usr/bin/env bash
# group: tools
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install uv

bin=$(proto bin uv)
assert_executable "$bin"
