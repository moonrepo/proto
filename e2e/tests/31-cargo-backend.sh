#!/usr/bin/env bash
# group: backends
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

install_backend cargo cargo-dist 0.31
