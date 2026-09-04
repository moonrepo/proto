#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$E2E_DIR/fixtures/basic.prototools" "$work/.prototools"
cd "$work"

run_probe proto run node -- --version
echo_probe
assert_probe_ok
assert_contains "$RUN_ALL" "v24"
