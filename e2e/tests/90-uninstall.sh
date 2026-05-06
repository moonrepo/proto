#!/usr/bin/env bash
# requires: 17-install-uv
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# uv was installed without dependents — safe to remove.
proto uninstall uv --yes

# uv should no longer be findable
if proto bin uv >/dev/null 2>&1; then
  fail "uv still resolvable after uninstall"
fi
