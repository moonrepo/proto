#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cd "$work"

# pin → writes .prototools with the version
proto pin node 22
assert_file ".prototools"
assert_contains "$(cat .prototools)" "node"

# unpin → removes the entry
proto unpin node
content=$(cat .prototools 2>/dev/null || echo "")
assert_not_contains "$content" 'node ='

# alias → registers a named alias for a version
proto alias node my-alias 22
out=$(proto alias node 2>&1 || true)
assert_contains "$out" "my-alias"

# unalias → removes the alias
proto unalias node my-alias
out=$(proto alias node 2>&1 || true)
assert_not_contains "$out" "my-alias"
