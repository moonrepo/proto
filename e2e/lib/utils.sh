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

  local bin="" bin_rc=0
  bin=$(proto bin "$context")
  bin_rc=$?

  echo "  exit=$bin_rc"
  echo "  path=$bin"

  if [[ $bin_rc -ne 0 ]]; then
    fail "bin '$bin' exited $bin_rc"
  fi

  assert_executable "$bin"

  echo "Verifying bin version..."

  local ver="" bin_rc=0
  ver=$("$bin" "$version_arg" 2>&1)
  bin_rc=$?

  echo "  exit=$bin_rc"
  echo "  output=$ver"

  if [[ $bin_rc -ne 0 ]]; then
    fail "bin '$bin' exited $bin_rc"
  fi

  # Ignore versions that contain a scope,
  # as the scope is typically not included in the outputs
  if [[ $version =~ ^[0-9] ]]; then
    assert_contains "$ver" "$version"
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

  local shim="" shim_rc=0
  shim=$(command -v "$exe_name")
  shim_rc=$?

  echo "  exit=$shim_rc"
  echo "  path=$shim"

  if [[ $shim_rc -ne 0 ]]; then
    fail "shim '$shim' exited $shim_rc"
  fi

  assert_executable "$shim"

  echo "Verifying shim version..."

  local ver="" shim_rc=0
  ver=$("$shim" "$version_arg" 2>&1)
  shim_rc=$?

  echo "  exit=$shim_rc"
  echo "  output=$ver"

  if [[ $shim_rc -ne 0 ]]; then
    fail "shim '$shim' exited $shim_rc"
  fi

  assert_contains "$ver" "$version"

  unset PROTO_DEBUG_SHIM
}

install_tool() {
  local tool="$1"
  local version="$2"
  local version_arg="${3:---version}"

  echo "Installing tool $tool $version..."

  retry 3 proto install "$tool" "$version" --pin local --log trace
  local exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    return $exit_code
  fi

  test_bin "$tool" "$version" "$version_arg"

  if [[ "$tool" != "rust" && "$tool" != "jdk" && "$tool" != "jre" ]]; then
    test_shim "$tool" "$version" "$version_arg"
  fi

  return $exit_code
}

install_backend() {
  local id="$1"
  local version="$2"
  local version_arg="${3:---version}"
  local context=""

  read -r context exe_name <<< $(parse_context "$id")

  echo "Installing backend tool $context $version..."

  retry 3 proto install "$context" "$version" --pin local --log trace
  local exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    return $exit_code
  fi

  test_bin "$id" "$version" "$version_arg"
  test_shim "$id" "$version" "$version_arg"

  return $exit_code
}
