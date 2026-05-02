#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# proto activate <shell> emits shell code that proto users `eval`/source from
# their shell rc. Verify the output is non-empty, mentions proto, and is
# actually evaluable without errors.
hook=$(proto activate bash 2>&1)
[[ -n "$hook" ]] || fail "proto activate bash produced no output"
assert_contains "$hook" "proto"

eval "$hook"
