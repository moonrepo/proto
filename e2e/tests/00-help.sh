#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

run_probe proto --version
echo_probe
assert_probe_ok
assert_contains "$RUN_ALL" "proto"

run_probe proto --help
echo_probe
assert_probe_ok
assert_contains "$RUN_ALL" "install"
assert_contains "$RUN_ALL" "run"
assert_contains "$RUN_ALL" "pin"
assert_contains "$RUN_ALL" "bin"
