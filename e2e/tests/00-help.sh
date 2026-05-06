#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

ver=$(proto --version 2>&1)
assert_contains "$ver" "proto"

help=$(proto --help 2>&1)
assert_contains "$help" "install"
assert_contains "$help" "run"
assert_contains "$help" "pin"
assert_contains "$help" "bin"
