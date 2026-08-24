#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# `proto deactivate <shell>` emits shell code that reverses a previous
# activation. Verify a full activate -> deactivate round trip within a
# single shell, driven by the `proto_deactivate` that the activation
# hook defines.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/.prototools" <<'EOF'
node = "24"

[env]
E2E_DEACTIVATE_VAR = "set-by-proto"

[shell.aliases]
e2e-alias = "node --version"
EOF

cd "$work"

path_before="$PATH"
eval "$(proto activate bash)"

# The activation must have applied everything we later assert on
assert_eq "${E2E_DEACTIVATE_VAR:-}" "set-by-proto"
assert_contains "$_PROTO_ACTIVATED_ENV" "E2E_DEACTIVATE_VAR"
assert_eq "$_PROTO_ACTIVATED_ALIASES" "e2e-alias"
assert_contains "$PATH" "activate-start"
assert_contains "$(alias e2e-alias 2>&1)" "node --version"
[[ -n "$(declare -F proto_activate)" ]] || fail "activation hook was not defined"

# Deactivating is expected to be evaluable without errors
proto_deactivate

assert_eq "${E2E_DEACTIVATE_VAR:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_ENV:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_ALIASES:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_PATH:-<unset>}" "<unset>"
assert_not_contains "$PATH" "activate-start"
assert_not_contains "$PATH" "activate-stop"
assert_not_contains "$(alias e2e-alias 2>&1)" "node --version"

# Activation only injects between the markers, so dropping them restores
# the exact list we started with, including any proto paths the profile set
assert_eq "$PATH" "$path_before"

# Both functions are removed, and the hook no longer runs on `cd`
[[ -z "$(declare -F proto_activate)" ]] || fail "activation hook was not removed"
[[ -z "$(declare -F proto_deactivate)" ]] || fail "deactivation hook was not removed"
assert_not_contains "${PROMPT_COMMAND:-}" "proto_activate"

# Deactivating without an activation is a no-op, not an error
out=$(proto deactivate bash --export)
assert_eq "$out" ""
