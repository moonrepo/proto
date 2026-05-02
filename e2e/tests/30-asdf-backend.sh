#!/usr/bin/env bash
# os: linux,macos
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# zig is a small, stable asdf-backed tool used in the existing Rust integration
# tests (see crates/cli/tests/install_one_backend_test.rs).
retry 3 proto install asdf:zig 0.13.0

out=$(proto status 2>&1)
assert_contains "$out" "asdf:zig"
