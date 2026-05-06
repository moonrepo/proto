#!/usr/bin/env bash
# Assertion + retry helpers. Sourced by every test.
#
# IMPORTANT: helpers `return 1` explicitly rather than rely on `set -e`. Bash
# does NOT propagate `set -e` failures inside `if`, `&&`, `||`, so a test that
# wraps an assertion in a conditional would silently pass.

fail() {
  echo "ASSERT FAIL: $*" >&2
  return 1
}

assert_eq() {
  [[ "$1" == "$2" ]] || fail "expected '$2', got '$1'"
}

assert_neq() {
  [[ "$1" != "$2" ]] || fail "expected value to differ from '$2'"
}

assert_contains() {
  [[ "$1" == *"$2"* ]] || fail "expected substring '$2' in: $1"
}

assert_not_contains() {
  [[ "$1" != *"$2"* ]] || fail "unexpected substring '$2' in: $1"
}

# Run a command, assert its exit code matches.
assert_exit() {
  local want=$1; shift
  local got=0
  "$@" || got=$?
  [[ $got -eq $want ]] || fail "expected exit $want, got $got from: $*"
}

assert_file() {
  [[ -f "$1" ]] || fail "missing file: $1"
}

assert_dir() {
  [[ -d "$1" ]] || fail "missing dir: $1"
}

assert_executable() {
  [[ -n "$1" ]] || fail "empty path passed to assert_executable"
  [[ -x "$1" || -f "$1.exe" || "$1" == *.cmd ]] || fail "not executable: $1"
}

# Retry a command with backoff. Use only for network-bound install commands.
# Usage: retry 3 proto install node 22.11.0
retry() {
  local n=${1:-3}; shift
  local i
  for ((i=1; i<=n; i++)); do
    if "$@"; then
      return 0
    fi
    if [[ $i -lt $n ]]; then
      echo "retry: attempt $i/$n failed, sleeping $((i*5))s before retry: $*" >&2
      sleep $((i*5))
    fi
  done
  return 1
}
