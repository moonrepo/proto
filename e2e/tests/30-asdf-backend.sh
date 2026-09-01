#!/usr/bin/env bash
# os: linux,macos
# group: backends
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

# Keep this on a tool proto does NOT ship as a built-in. Backend tools share the
# inventory dir and shim name of their id (unless the backend opts into
# `scoped_backend_dir`), so e.g. `asdf:zig` would collide with built-in `zig`
# in the suite's shared PROTO_HOME.
install_backend asdf:jq 1.7
