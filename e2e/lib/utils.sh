#!/usr/bin/env bash
set -euo pipefail
source ./env.sh
source ./assert.sh

install_tool() {
  tool="$1"
  version="$2"
  version_arg="${3:---version}"

  retry 3 proto install "$tool" "$version" --pin local
  exit_code=$?

  if [ $exit_code -ne 0 ]; then
    return $exit_code
  fi

  bin=$(proto bin "$tool")
  assert_executable "$bin"

  ver=$("$bin" "$version_arg" 2>&1)
  assert_contains "$ver" "$version"

  return $exit_code
}
