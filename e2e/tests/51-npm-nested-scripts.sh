#!/usr/bin/env bash
# requires: 20-install-npm
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# npm scripts that invoke `npm` again must resolve to the proto managed npm,
# not the npm bundled with node. The bundled npm wins when node's bin
# directory comes before npm's directories within the `PATH` created by
# `proto run`, which surfaces as scripts running with the wrong npm version.
# https://github.com/moonrepo/proto/issues/946
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
printf 'node = "24"\nnpm = "11.13"\n' > "$work/.prototools"
cat > "$work/package.json" <<'EOF'
{
  "name": "e2e-nested-scripts",
  "scripts": {
    "inner": "npm --version",
    "outer": "npm run -s inner"
  }
}
EOF
cd "$work"

direct=$(npm --version 2>&1 | tr -d '\r')
one_level=$(npm run -s inner 2>&1 | tr -d '\r')
two_level=$(npm run -s outer 2>&1 | tr -d '\r')

# Anchor against every invocation resolving the bundled npm
assert_contains "$direct" "11.13"

assert_eq "$one_level" "$direct"
assert_eq "$two_level" "$direct"
