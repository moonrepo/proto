#!/usr/bin/env bash
# requires: 10-install-node 11-install-bun 13-install-go 14-install-python
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$E2E_DIR/fixtures/multi.prototools" "$work/.prototools"
cd "$work"

assert_contains "$(proto run node   -- --version 2>&1)" "v24"
assert_contains "$(proto run bun    -- --version 2>&1)" "1.2"
assert_contains "$(proto run go     -- version   2>&1)" "go1.23"
assert_contains "$(proto run python -- --version 2>&1)" "Python 3.12"
