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

# Report what died and where, for any failure the caller does not handle.
# Without this, `set -e` aborts the test silently and its log just stops
# mid-way, with no exit code and no message.
#
# Failures inside a command substitution are skipped: they are either handled
# by the caller (`out=$(cmd) || rc=$?`) or reported again by the assignment
# that surrounds them, and reporting both is just noise.
_report_failure() {
  local rc=$?

  if [[ ${BASH_SUBSHELL:-0} -ne 0 ]]; then
    return 0
  fi

  echo "COMMAND FAILED (exit=$rc) at ${BASH_SOURCE[1]}:${BASH_LINENO[0]}: $BASH_COMMAND" >&2

  local i
  for ((i = 1; i < ${#FUNCNAME[@]} - 1; i++)); do
    echo "  called from ${FUNCNAME[$i]} (${BASH_SOURCE[$i + 1]}:${BASH_LINENO[$i]})" >&2
  done
}

# `errtrace` so the trap also covers failures inside functions, which is where
# every assertion and helper lives
set -o errtrace
trap _report_failure ERR

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

# Run a command and record its result in RUN_RC / RUN_OUT / RUN_ERR, keeping
# stdout and stderr separate. Never aborts the caller, so the result can be
# logged before it is asserted on.
#
# This exists because `out=$(cmd)` under `set -e` aborts the test the moment
# the command fails, before any diagnostic can be printed, which turns a
# failing tool into a log that simply stops.
run_probe() {
  local err_file
  err_file=$(mktemp)

  RUN_CMD="$*"
  RUN_RC=0
  RUN_OUT=$("$@" 2>"$err_file") || RUN_RC=$?
  RUN_ERR=$(cat "$err_file")

  rm -f "$err_file"

  # Tools are inconsistent about which stream they report on, so most
  # assertions want both
  if [[ -n "$RUN_ERR" ]]; then
    RUN_ALL="$RUN_OUT
$RUN_ERR"
  else
    RUN_ALL="$RUN_OUT"
  fi
}

# Echo what a `run_probe` saw, for the test log.
echo_probe() {
  echo "\$ $RUN_CMD"
  echo "  exit=$RUN_RC"
  echo "  ${1:-output}=$RUN_OUT"

  if [[ -n "$RUN_ERR" ]]; then
    echo "  stderr=$RUN_ERR"
  fi
}

# Assert the last `run_probe` succeeded, naming the command that didn't.
assert_probe_ok() {
  [[ $RUN_RC -eq 0 ]] || fail "\`$RUN_CMD\` exited $RUN_RC"
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
