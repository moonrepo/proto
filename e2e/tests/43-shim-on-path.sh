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

ver=$(node --version 2>&1)
assert_contains "$ver" "v24"

bin=$(which node 2>&1)
assert_contains "$bin" "$PROTO_HOME/shims"
