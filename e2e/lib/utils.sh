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

  echo "Verifying bin is executable..."

  bin=$(proto bin "$backend:$tool")
  assert_executable "$bin"

  # Bin
  echo "Verifying bin version..."

  ver=$("$bin" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  # Shim
  echo "Verifying shim version..."

  ver=$("$bin_name" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  return $exit_code
}
