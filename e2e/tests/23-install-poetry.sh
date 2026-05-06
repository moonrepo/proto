#!/usr/bin/env bash
# requires: 14-install-python
# group: tools-secondary
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_tool poetry 1.8
