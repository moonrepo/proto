#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# Final teardown: clean removes unused/cached items. Just verify it runs.
proto clean --yes 2>&1
