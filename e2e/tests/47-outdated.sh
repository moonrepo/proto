#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp "$E2E_DIR/fixtures/basic.prototools" "$work/.prototools"
cd "$work"

# `proto outdated` may exit non-zero when out-of-date tools are detected; we
# only care that the command runs and produces some output.
out=$(proto outdated 2>&1 || true)
[[ -n "$out" ]] || fail "proto outdated produced no output"
