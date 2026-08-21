#!/usr/bin/env bash
# E2E test harness for proto.
#
# Walks tests/*.sh in glob order, parses `# requires:`, `# os:`, `# group:`
# directives, and runs each test as a subprocess with shared PROTO_HOME.
# Consecutive tests sharing a `# group:` value run as background jobs in
# parallel; the harness waits for the group to complete before moving on.
# Captures stdout/stderr to .logs/, exits non-zero if any test failed.
#
# Usage:
#   ./e2e/run.sh             # run all tests
#   ./e2e/run.sh install     # only tests whose name contains "install"
#
# Env knobs:
#   E2E_SERIAL=1        # never run a group in parallel, one test at a time
#   E2E_TAIL=200        # lines of a failing test's log to print inline
#   E2E_KEEP_SCRATCH=1  # keep the shared work dir for post-mortem poking
set -euo pipefail

source "$(dirname "$0")/lib/env.sh"

# Verify the proto binary is reachable
if ! command -v proto >/dev/null 2>&1 || ! proto --version >/dev/null 2>&1; then
  {
    echo "FATAL: proto binary not available on PATH."
    echo "       PATH=$PATH"
    echo "       Run e2e/bootstrap.sh first to install the binary into PROTO_HOME."
  } >&2
  exit 1
fi

# State is intentionally kept between runs: bootstrap.sh puts the binary under
# test into the store, so wiping it here would delete what we are testing. To
# start from scratch, remove the store and re-bootstrap:
#   rm -rf e2e/.proto-home && ./e2e/bootstrap.sh

# Neutral cwd for tests that don't need a specific work dir. Lives outside
# the repo so the repo's own .prototools doesn't get picked up as ancestry.
E2E_SCRATCH="$(mktemp -d)"
export E2E_SCRATCH

if [[ -z "${E2E_KEEP_SCRATCH:-}" ]]; then
  trap 'rm -rf "$E2E_SCRATCH"' EXIT
fi

E2E_TAIL="${E2E_TAIL:-40}"

# Stale post-mortems from an earlier run would be read as belonging to this one
rm -f "$E2E_LOGS"/*.postmortem.txt

echo "Using proto at: $(command -v proto)"
echo "With version: $(proto --version)"
echo ""
echo "OS:         $E2E_OS"
echo "PROTO_HOME: $PROTO_HOME"
echo "E2E_DIR:    $E2E_DIR"
echo "E2E_LOGS:   $E2E_LOGS"
echo "REPO_ROOT:  $REPO_ROOT"
echo "SCRATCH:    $E2E_SCRATCH"
echo "PATH:       $PATH"
echo ""

filter="${1:-}"

tests=()
for f in "$E2E_DIR/tests/"*.sh; do
  [[ -f "$f" ]] || continue
  name=$(basename "$f" .sh)
  if [[ -n "$filter" && "$name" != *"$filter"* ]]; then
    continue
  fi
  tests+=("$f")
done

if [[ ${#tests[@]} -eq 0 ]]; then
  echo "No tests matched filter: $filter" >&2
  exit 1
fi

# Parallel arrays — bash 3.2 (macOS system) lacks associative arrays
test_names=()
test_status=()
test_reason=()

get_status() {
  local target="$1" i
  for ((i=0; i<${#test_names[@]}; i++)); do
    if [[ "${test_names[$i]}" == "$target" ]]; then
      echo "${test_status[$i]}"
      return 0
    fi
  done
  echo ""
}

record() {
  test_names+=("$1")
  test_status+=("$2")
  test_reason+=("${3:-}")
}

# Reads the first `# <key>: <value>` line from a file. Pure bash to avoid
# SIGPIPE issues with `set -o pipefail`.
parse_directive() {
  local file="$1" key="$2" line val
  while IFS= read -r line; do
    if [[ "$line" == "# $key:"* ]]; then
      val="${line#"# $key:"}"
      val="${val#"${val%%[![:space:]]*}"}"  # trim leading whitespace
      echo "$val"
      return 0
    fi
  done < "$file"
}

# Returns "" if the test should run, otherwise records a SKIP and returns the
# reason. Used by both run_one and run_batch to enforce os/requires gates
# uniformly.
should_skip() {
  local file="$1" name="$2"
  local os_list; os_list=$(parse_directive "$file" "os" || true)
  local requires; requires=$(parse_directive "$file" "requires" || true)

  if [[ -n "$os_list" && ",$os_list," != *",$E2E_OS,"* ]]; then
    echo "os=$os_list"
    return 0
  fi

  if [[ -n "$requires" ]]; then
    local req st
    for req in $requires; do
      st=$(get_status "$req")
      if [[ "$st" == "FAIL" || "$st" == "SKIP" ]]; then
        echo "dep $req=$st"
        return 0
      fi
    done
  fi

  echo ""
}

# Dump the state a failing test left behind, while it still exists. The shared
# config and the shim registry are behind most cross-test failures, and neither
# shows up in the test's own log.
write_postmortem() {
  local name="$1"
  local file="$E2E_LOGS/$name.postmortem.txt"

  {
    echo "===== $name post-mortem ====="
    echo ""
    echo "--- $E2E_SCRATCH/.prototools"
    cat "$E2E_SCRATCH/.prototools" 2>/dev/null || echo "(missing)"
    echo ""
    echo "--- $_PROTO_HOME_POSIX/shims/registry.json"
    cat "$_PROTO_HOME_POSIX/shims/registry.json" 2>/dev/null || echo "(missing)"
    echo ""
    echo "--- shims dir"
    ls -l "$_PROTO_HOME_POSIX/shims" 2>/dev/null || echo "(missing)"
    echo ""
    echo "--- proto status"
    (cd "$E2E_SCRATCH" && proto status 2>&1) || true
  } >"$file"

  echo "$file"
}

# Lead with what actually died, then the tail, then where to find the state.
# A bare tail buries the cause under whatever trace logging came after it.
report_failure() {
  local name="$1" log="$2" indent="${3:-}"
  local cause
  cause=$(grep -nE "ASSERT FAIL|COMMAND FAILED|FATAL" "$log" 2>/dev/null | tail -5 || true)

  if [[ -n "$cause" ]]; then
    echo "${indent}----- cause $name -----"
    echo "$cause" | sed "s/^/${indent}/"
  fi

  echo "${indent}----- tail $name (last $E2E_TAIL lines) -----"
  tail -"$E2E_TAIL" "$log" 2>/dev/null | sed "s/^/${indent}/" || true
  echo "${indent}----- end -----"
  echo "${indent}post-mortem: $(write_postmortem "$name")"
}

run_one() {
  local file="$1"
  local name; name=$(basename "$file" .sh)
  local skip; skip=$(should_skip "$file" "$name")

  if [[ -n "$skip" ]]; then
    record "$name" "SKIP" "$skip"
    printf "[SKIP] %-32s  %s\n" "$name" "$skip"
    return 0
  fi

  printf "[RUN]  %-32s\n" "$name"
  local log="$E2E_LOGS/$name.log"
  local start=$SECONDS rc=0
  ( cd "$E2E_SCRATCH" && bash "$file" ) >"$log" 2>&1 || rc=$?
  local dur=$((SECONDS - start))

  if [[ $rc -eq 0 ]]; then
    record "$name" "PASS" ""
    printf "[PASS] %-32s  %ds\n" "$name" "$dur"
  else
    record "$name" "FAIL" "exit=$rc"
    printf "[FAIL] %-32s  %ds exit=%d  log=%s\n" "$name" "$dur" "$rc" "$log"
    report_failure "$name" "$log"
  fi
}

# Runs a batch of tests concurrently as background jobs. Members must be
# independent of each other — `# requires:` should only point to tests in
# earlier groups or earlier sequential tests, never to peers in the same
# group (peer status isn't visible until the batch completes).
run_batch() {
  local group="$1"; shift
  local files=("$@")
  printf "[GRP]  %s  (%d tests, parallel)\n" "$group" "${#files[@]}"

  local pids=() names=() logs=() starts=()
  local f name skip log

  for f in "${files[@]}"; do
    name=$(basename "$f" .sh)
    skip=$(should_skip "$f" "$name")
    if [[ -n "$skip" ]]; then
      record "$name" "SKIP" "$skip"
      printf "  [SKIP] %-30s  %s\n" "$name" "$skip"
      continue
    fi

    log="$E2E_LOGS/$name.log"
    starts+=("$SECONDS")
    ( cd "$E2E_SCRATCH" && bash "$f" >"$log" 2>&1 ) &
    pids+=("$!")
    names+=("$name")
    logs+=("$log")
    printf "  [RUN]  %-30s  pid=%d\n" "$name" "$!"
  done

  local i pid rc start dur
  for ((i=0; i<${#pids[@]}; i++)); do
    pid="${pids[$i]}"
    name="${names[$i]}"
    log="${logs[$i]}"
    start="${starts[$i]}"
    rc=0
    wait "$pid" || rc=$?
    dur=$((SECONDS - start))

    if [[ $rc -eq 0 ]]; then
      record "$name" "PASS" ""
      printf "  [PASS] %-30s  %ds\n" "$name" "$dur"
    else
      record "$name" "FAIL" "exit=$rc"
      printf "  [FAIL] %-30s  %ds exit=%d  log=%s\n" "$name" "$dur" "$rc" "$log"
      report_failure "$name" "$log" "  "
    fi
  done
}

# Dispatcher: walk tests in glob order, batching adjacent same-group runs.
batch=()
batch_group=""

flush_batch() {
  if [[ ${#batch[@]} -eq 0 ]]; then
    return 0
  fi
  if [[ ${#batch[@]} -eq 1 ]]; then
    run_one "${batch[0]}"
  else
    run_batch "$batch_group" "${batch[@]}"
  fi
  batch=()
  batch_group=""
}

for f in "${tests[@]}"; do
  group=$(parse_directive "$f" "group" || true)

  # Every test becomes its own batch, so nothing runs concurrently. Use this to
  # tell a real failure apart from one test interfering with its group peers.
  if [[ -n "${E2E_SERIAL:-}" ]]; then
    group=""
  fi

  if [[ -n "$group" && "$group" == "$batch_group" ]]; then
    batch+=("$f")
  else
    flush_batch
    batch=("$f")
    batch_group="$group"
  fi
done
flush_batch

# The GitHub Actions run page showed nothing beyond "Process completed with exit
# code 1" — not even which test failed. Annotations put the cause on the page,
# and the step summary lists what failed or was skipped without downloading the
# log artifact.
write_ci_report() {
  local i name status reason log cause

  for ((i = 0; i < ${#test_names[@]}; i++)); do
    if [[ "${test_status[$i]}" != "FAIL" ]]; then
      continue
    fi

    name="${test_names[$i]}"
    log="$E2E_LOGS/$name.log"
    cause=$(grep -hE "ASSERT FAIL|COMMAND FAILED|FATAL" "$log" 2>/dev/null | head -1 || true)

    echo "::error title=E2E ($E2E_OS) $name::${cause:-no assertion message, see $name.log}"
  done

  if [[ -z "${GITHUB_STEP_SUMMARY:-}" ]]; then
    return 0
  fi

  {
    echo "### E2E ($E2E_OS): $pass passed, $fail failed, $skip skipped"
    echo ""

    for ((i = 0; i < ${#test_names[@]}; i++)); do
      name="${test_names[$i]}"
      status="${test_status[$i]}"
      reason="${test_reason[$i]}"

      case "$status" in
        FAIL)
          log="$E2E_LOGS/$name.log"
          cause=$(grep -hE "ASSERT FAIL|COMMAND FAILED|FATAL" "$log" 2>/dev/null | head -1 || true)
          echo "- ❌ \`$name\` ($reason) — ${cause:-no assertion message}"
          ;;
        SKIP)
          echo "- ⏭️ \`$name\` ($reason)"
          ;;
      esac
    done
  } >>"$GITHUB_STEP_SUMMARY"
}

# Summary
pass=0; fail=0; skip=0
for ((i=0; i<${#test_names[@]}; i++)); do
  case "${test_status[$i]}" in
    PASS) pass=$((pass+1)) ;;
    FAIL) fail=$((fail+1)) ;;
    SKIP) skip=$((skip+1)) ;;
  esac
done

echo ""
echo "===== Summary ($E2E_OS) ====="
printf "  Passed:  %d\n" "$pass"
printf "  Failed:  %d\n" "$fail"
printf "  Skipped: %d\n" "$skip"
printf "  Total:   %d\n" "${#test_names[@]}"

if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
  write_ci_report
fi

echo ""
echo "Log files:"
ls -l "$E2E_LOGS"

if [[ $fail -gt 0 ]]; then
  echo ""
  echo "Failed tests:"
  for ((i=0; i<${#test_names[@]}; i++)); do
    if [[ "${test_status[$i]}" == "FAIL" ]]; then
      printf "  - %s (%s)\n" "${test_names[$i]}" "${test_reason[$i]}"
    fi
  done
  exit 1
fi

exit 0
