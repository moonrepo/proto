#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

install_tool() {
  tool="$1"
  version="$2"
  version_arg="${3:---version}"

  echo "Installing tool $tool $version..."

  retry 3 proto install "$tool" "$version" --pin local --log trace
  exit_code=$?

  if [ $exit_code -ne 0 ]; then
    return $exit_code
  fi

  # Bin
  echo "Verifying bin is executable..."

  bin=$(proto bin "$tool")
  echo "  $bin" # Debug
  assert_executable "$bin"

  echo "Verifying bin version..."

  bin_rc=0
  ver=$("$bin" "$version_arg" 2>&1) || bin_rc=$?
  echo "  exit=$bin_rc"
  echo "  $ver" # Debug
  [[ $bin_rc -eq 0 ]] || fail "bin '$bin' exited $bin_rc"
  assert_contains "$ver" "$version"

  # Shim
  if [[ "$tool" != "rust" ]]; then
    export PROTO_DEBUG_SHIM=1;

    echo "Verifying shim is executable..."

    shim=$(command -v "$tool")
    echo "  $shim" # Debug
    assert_executable "$shim"

    echo "Verifying shim version..."

    shim_rc=0
    ver=$("$tool" "$version_arg" 2>&1) || shim_rc=$?
    echo "  exit=$shim_rc"
    echo "  $ver" # Debug
    [[ $shim_rc -eq 0 ]] || fail "shim '$tool' exited $shim_rc"
    assert_contains "$ver" "$version"

    unset PROTO_DEBUG_SHIM
  fi

  return $exit_code
}

install_backend() {
  context="$1"
  version="$2"
  version_arg="${3:---version}"
  backend=""
  tool=""
  bin_name=""

  export IFS=":"
  count=0
  for part in $context; do
    if [ $count -eq 0 ]; then
      backend="$part"
    elif [ $count -eq 1 ]; then
      tool="$part"
      bin_name="$part"
    elif [ $count -eq 2 ]; then
      bin_name="$part"
    else
      echo "Invalid context: $context"
      exit 1
    fi

    (( count+=1 ))
  done

  echo "Installing backend tool $backend:$tool $version..."

  retry 3 proto install "$backend:$tool" "$version" --pin local --log trace
  exit_code=$?

  if [ $exit_code -ne 0 ]; then
    return $exit_code
  fi

  # Bin
  echo "Verifying bin is executable..."

  bin=$(proto bin "$backend:$tool")
  echo "  $bin" # Debug
  assert_executable "$bin"

  echo "Verifying bin version..."

  bin_rc=0
  ver=$("$bin" "$version_arg" 2>&1) || bin_rc=$?
  echo "  exit=$bin_rc"
  echo "  $ver" # Debug
  [[ $bin_rc -eq 0 ]] || fail "bin '$bin' exited $bin_rc"
  assert_contains "$ver" "$version"

  # Shim
  export PROTO_DEBUG_SHIM=1;

  echo "Verifying shim is executable..."

  shim=$(command -v "$bin_name")
  echo "  $shim" # Debug
  assert_executable "$shim"

  echo "Verifying shim version..."

  shim_rc=0
  ver=$("$bin_name" "$version_arg" 2>&1) || shim_rc=$?
  echo "  exit=$shim_rc"
  echo "  $ver" # Debug
  [[ $shim_rc -eq 0 ]] || fail "shim '$bin_name' exited $shim_rc"
  assert_contains "$ver" "$version"

  unset PROTO_DEBUG_SHIM

  return $exit_code
}
