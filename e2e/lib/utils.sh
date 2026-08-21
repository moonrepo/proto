#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

parse_context() {
  local context="$1"
  local backend=""
  local tool=""
  local exe=""

  export IFS=":"
  local count=0

  for part in $context; do
    if [ $count -eq 0 ]; then
      backend="$part"
      tool="$part"
      exe="$part"
    elif [ $count -eq 1 ]; then
      tool="$part"
      exe="$part"
    elif [ $count -eq 2 ]; then
      exe="$part"
    else
      echo "Invalid context: $context"
      exit 1
    fi

    (( count+=1 ))
  done

  if [[ $count -eq 1 ]]; then
    echo "$tool" "$exe"
  else
    echo "$backend:$tool" "$exe"
  fi
}

test_bin() {
  local id="$1"
  local version="$2"
  local version_arg="$3"
  local context=""
  local exe_name=""

  read -r context exe_name <<< $(parse_context "$id")

  echo "Verifying bin is executable..."

  run_probe proto bin "$context"
  echo_probe path
  assert_probe_ok

  local bin="$RUN_OUT"
  assert_executable "$bin"

  echo "Verifying bin version..."

  run_probe "$bin" "$version_arg"
  echo_probe
  assert_probe_ok

  # Ignore versions that contain a scope,
  # as the scope is typically not included in the outputs
  if [[ $version =~ ^[0-9] ]]; then
    assert_contains "$RUN_ALL" "$version"
  fi
}

test_shim() {
  export PROTO_DEBUG_SHIM=1;

  local id="$1"
  local version="$2"
  local version_arg="$3"
  local context=""
  local exe_name=""

  read -r context exe_name <<< $(parse_context "$id")

  echo "Verifying shim is executable..."

  run_probe command -v "$exe_name"
  echo_probe path

  if [[ $RUN_RC -ne 0 ]]; then
    fail "no shim named '$exe_name' found on PATH"
  fi

  local shim="$RUN_OUT"
  assert_executable "$shim"

  echo "Verifying shim version..."

  run_probe "$shim" "$version_arg"
  echo_probe
  assert_probe_ok
  assert_contains "$RUN_ALL" "$version"

  unset PROTO_DEBUG_SHIM
}

install_tool() {
  local tool="$1"
  local version="$2"
  local version_arg="${3:---version}"

  echo "Installing tool $tool $version..."

  retry 3 proto install "$tool" "$version" --pin local --log trace || return $?

  test_bin "$tool" "$version" "$version_arg"

  if [[ "$tool" != "rust" && "$tool" != "jdk" && "$tool" != "jre" ]]; then
    test_shim "$tool" "$version" "$version_arg"
  fi
}

install_backend() {
  local id="$1"
  local version="$2"
  local version_arg="${3:---version}"
  local context=""

  read -r context exe_name <<< $(parse_context "$id")

  echo "Installing backend tool $context $version..."

  retry 3 proto install "$context" "$version" --pin local --log trace || return $?

  test_bin "$id" "$version" "$version_arg"
  test_shim "$id" "$version" "$version_arg"
}
