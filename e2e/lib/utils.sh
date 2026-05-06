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

  echo "Verifying bin is executable..."

  bin=$(proto bin "$tool")
  assert_executable "$bin"

  # Bin
  echo "Verifying bin version..."

  ver=$("$bin" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  # Shim
  if [[ "$tool" != "rust" ]]; then
    echo "Verifying shim version..."

    ver=$("$tool" "$version_arg" 2>&1)
    assert_contains "$ver" "$version"
  fi

  return $exit_code
}

install_backend() {
  backend="$1"
  tool="$2"
  version="$3"
  version_arg="${4:---version}"

  echo "Installing backend tool $backend:$tool $version..."

  retry 3 proto install "$backend:$tool" "$version" --pin local --log trace
  exit_code=$?

  if [ $exit_code -ne 0 ]; then
    return $exit_code
  fi

  echo "Verifying bin is executable..."

  bin=$(proto bin "$backend:$tool")
  assert_executable "$bin"

  # Bin
  echo "Verifying bin version..."

  ver=$("$bin" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  # Shim
  echo "Verifying shim version..."

  ver=$("$tool" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  return $exit_code
}
