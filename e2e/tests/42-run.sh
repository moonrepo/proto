#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Use a workdir with a multi-tool .prototools so proto picks the pinned
# versions and `proto run` resolves deterministically.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$E2E_DIR/fixtures/multi.prototools" "$work/.prototools"
cd "$work"

# Helper: only assert if the tool actually got installed upstream.
check_run() {
  local tool="$1" expect="$2"
  shift 2
  if proto bin "$tool" >/dev/null 2>&1; then
    out=$(proto run "$tool" -- "$@" 2>&1)
    assert_contains "$out" "$expect"
  fi
}

check_run node   "v24"          --version
check_run bun    "1.2"          --version
check_run go     "go1.23"       version
check_run python "Python 3.12"  --version
