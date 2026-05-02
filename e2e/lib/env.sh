#!/usr/bin/env bash
# Sourced by run.sh and every test. Sets up cross-platform env so the same
# bash code runs on Linux, macOS, and Windows (Git Bash).

# Resolve repo root relative to this file
_lib_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
E2E_DIR="$(cd "$_lib_dir/.." && pwd)"
REPO_ROOT="$(cd "$E2E_DIR/.." && pwd)"
export E2E_DIR REPO_ROOT

# Detect OS
case "$OSTYPE" in
  linux*)              E2E_OS=linux ;;
  darwin*)             E2E_OS=macos ;;
  msys*|cygwin*|win32) E2E_OS=windows ;;
  *) echo "env.sh: unsupported OSTYPE: $OSTYPE" >&2; exit 1 ;;
esac
export E2E_OS

# On Git Bash, MSYS auto-translates POSIX-looking strings in argv to Windows
# paths when invoking native .exe binaries. That mangles plugin IDs, URLs,
# and any argv that legitimately starts with /. Disable it.
if [[ "$E2E_OS" == "windows" ]]; then
  export MSYS_NO_PATHCONV=1
  export MSYS2_ARG_CONV_EXCL='*'
fi

# Where run.sh writes per-test stdout/stderr captures
export E2E_LOGS="$E2E_DIR/.logs"

# Shared PROTO_HOME for the whole run. Tests accumulate state here.
# Keep an internal POSIX form for bash builtins / PATH composition,
# and export PROTO_HOME in the form proto's binary expects.
if [[ -z "${_PROTO_HOME_POSIX:-}" ]]; then
  _PROTO_HOME_POSIX="$HOME/.proto"
fi

if [[ "$E2E_OS" == "windows" ]]; then
  # Mixed (forward-slash) form: accepted by Windows APIs and safe in bash.
  PROTO_HOME="$(cygpath -m "$_PROTO_HOME_POSIX")"
else
  PROTO_HOME="$_PROTO_HOME_POSIX"
fi

export PROTO_HOME
export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH"

# Stable locale across runners (stderr matching shouldn't depend on it)
export LANG="${LANG:-C.UTF-8}"
export LC_ALL="${LC_ALL:-C.UTF-8}"

# Match existing CI diagnostics
export PROTO_DEBUG_COMMAND=1
export PROTO_DEBUG_WASM=1
export RUST_BACKTRACE=1
