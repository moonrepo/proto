#!/usr/bin/env bash
# os: linux,macos
# group: backends
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_backend asdf zig 0.13
