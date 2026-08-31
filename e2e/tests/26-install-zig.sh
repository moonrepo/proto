#!/usr/bin/env bash
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_tool zig 0.16
install_tool zls 0.16
