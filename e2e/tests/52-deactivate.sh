#!/usr/bin/env bash
# requires: 10-install-node
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

# `proto deactivate <shell>` emits shell code that reverses a previous
# activation. Verify a full activate -> deactivate round trip within a
# single shell, driven by the `_proto_deactivate_hook` that the activation
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

eval "$(proto activate bash)"

# The activation must have applied everything we later assert on
assert_eq "${E2E_DEACTIVATE_VAR:-}" "set-by-proto"
assert_contains "$_PROTO_ACTIVATED_ENV" "E2E_DEACTIVATE_VAR"
assert_eq "$_PROTO_ACTIVATED_ALIASES" "e2e-alias"
assert_contains "$PATH" "activate-start"
assert_contains "$(alias e2e-alias 2>&1)" "node --version"
[[ -n "$(declare -F _proto_activate_hook)" ]] || fail "activation hook was not defined"

# Deactivating is expected to be evaluable without errors
_proto_deactivate_hook

assert_eq "${E2E_DEACTIVATE_VAR:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_ENV:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_ALIASES:-<unset>}" "<unset>"
assert_eq "${_PROTO_ACTIVATED_PATH:-<unset>}" "<unset>"
assert_not_contains "$PATH" "activate-start"
assert_not_contains "$PATH" "activate-stop"
assert_not_contains "$(alias e2e-alias 2>&1)" "node --version"

# Both functions are removed, and the hook no longer runs on `cd`
[[ -z "$(declare -F _proto_activate_hook)" ]] || fail "activation hook was not removed"
[[ -z "$(declare -F _proto_deactivate_hook)" ]] || fail "deactivation hook was not removed"
assert_not_contains "${PROMPT_COMMAND:-}" "_proto_activate_hook"

# Deactivating without an activation is a no-op, not an error
out=$(proto deactivate bash --export)
assert_eq "$out" ""
