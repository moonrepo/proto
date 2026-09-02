#!/usr/bin/env bash
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

# shfmt is provided by the remote community registry, not the built-in plugin
# list. Installing it without a configured locator verifies community lookup.
install_tool shfmt 3.14.0
