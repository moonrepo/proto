#!/usr/bin/env bash
# requires: 14-install-python
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

retry 3 proto install poetry 1.8

bin=$(proto bin poetry)
assert_executable "$bin"
