#!/usr/bin/env bash
# requires: 10-install-node
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_tool pnpm 10.1
