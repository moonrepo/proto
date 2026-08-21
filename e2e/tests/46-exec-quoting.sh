#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Argv passthrough through real shell quoting — spaces, $-prefixed strings.
# This is what the Rust unit tests can't fully exercise (they build the argv
# vector directly).
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
echo 'node = "24"' > "$work/.prototools"
cd "$work"

run_probe proto run node -- -e "console.log('hello world with literal \$dollar')"
echo_probe
assert_probe_ok
assert_contains "$RUN_ALL" 'hello world with literal $dollar'
