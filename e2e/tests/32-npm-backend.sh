#!/usr/bin/env bash
# group: backends
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_backend npm:typescript:tsc 6

# With scope
install_backend npm:@moonrepo/cli:moon 2
