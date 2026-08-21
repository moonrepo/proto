#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Bare command, no `proto run` — exercises real OS launcher behavior including
# Windows PATHEXT (.exe) resolution. PATH already includes $PROTO_HOME/shims
# via lib/env.sh.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
echo 'node = "24"' > "$work/.prototools"
cd "$work"

run_probe node --version
echo_probe
assert_probe_ok
assert_contains "$RUN_ALL" "v24"

run_probe which node
echo_probe path
assert_probe_ok
assert_contains "$RUN_OUT" "$_PROTO_HOME_POSIX/shims"
