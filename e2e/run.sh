#!/usr/bin/env bash
# E2E test harness for proto.
#
# Walks tests/*.sh in glob order, parses `# requires:` and `# os:` directives,
# runs each as a subprocess with shared PROTO_HOME, captures stdout/stderr to
# .logs/, and exits non-zero if any test failed.
#
# Usage:
#   ./e2e/run.sh             # run all tests
#   ./e2e/run.sh install     # only tests whose name contains "install"
set -euo pipefail

source "$(dirname "$0")/lib/env.sh"

# Verify the proto binary is reachable
if ! command -v proto >/dev/null 2>&1 || ! proto --version >/dev/null 2>&1; then
  {
    echo "FATAL: proto binary not available on PATH."
    echo "       PROTO_BIN_DIR=$PROTO_BIN_DIR"
    echo "       PATH=$PATH"
    echo "       Did you run 'cargo build --release --bin proto --bin proto-shim'?"
  } >&2
  exit 1
fi

echo "Using proto at: $(command -v proto)"
proto --version
echo "OS:        $E2E_OS"
echo "PROTO_HOME: $PROTO_HOME"
echo ""

# Wipe state from any previous run
rm -rf "$_PROTO_HOME_POSIX" "$E2E_LOGS"
mkdir -p "$_PROTO_HOME_POSIX" "$E2E_LOGS"

# Neutral cwd for tests that don't need a specific work dir. Lives outside
# the repo so the repo's own .prototools doesn't get picked up as ancestry.
E2E_SCRATCH="$(mktemp -d)"
trap 'rm -rf "$E2E_SCRATCH"' EXIT
export E2E_SCRATCH

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

run_one() {
  local file="$1"
  local name; name=$(basename "$file" .sh)
  local os_list; os_list=$(parse_directive "$file" "os" || true)
  local requires; requires=$(parse_directive "$file" "requires" || true)

  if [[ -n "$os_list" && ",$os_list," != *",$E2E_OS,"* ]]; then
    record "$name" "SKIP" "os=$os_list"
    printf "[SKIP] %-32s  os mismatch (%s)\n" "$name" "$os_list"
    return 0
  fi

  if [[ -n "$requires" ]]; then
    local req st blocker=""
    for req in $requires; do
      st=$(get_status "$req")
      if [[ "$st" == "FAIL" || "$st" == "SKIP" ]]; then
        blocker="$req=$st"
        break
      fi
    done
    if [[ -n "$blocker" ]]; then
      record "$name" "SKIP" "dep $blocker"
      printf "[SKIP] %-32s  dep %s\n" "$name" "$blocker"
      return 0
    fi
  fi

  printf "[RUN ] %-32s\n" "$name"
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
    echo "----- tail $name -----"
    tail -40 "$log" || true
    echo "----- end -----"
  fi
}

for f in "${tests[@]}"; do
  run_one "$f"
done

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
