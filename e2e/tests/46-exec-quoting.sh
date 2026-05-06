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

out=$(proto run node -- -e "console.log('hello world with literal \$dollar')" 2>&1)
assert_contains "$out" 'hello world with literal $dollar'
