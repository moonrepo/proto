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

# Where run.sh writes per-test stdout/stderr captures.
export E2E_LOGS="$E2E_DIR/.logs"
mkdir -p "$E2E_LOGS"

# Shared PROTO_HOME for the whole run. Tests accumulate state here.
# Keep an internal POSIX form for bash builtins / PATH composition,
# and export PROTO_HOME in the form proto's binary expects.
#
# This lives inside the repo (and is gitignored) so that a local run cannot
# touch the developer's own store: the suite installs 20+ tools, uninstalls
# one, and runs `proto clean` over whatever it finds. Set E2E_USE_REAL_HOME=1
# to point it back at ~/.proto.
if [[ -z "${_PROTO_HOME_POSIX:-}" ]]; then
  if [[ -n "${E2E_USE_REAL_HOME:-}" ]]; then
    _PROTO_HOME_POSIX="$HOME/.proto"
  else
    _PROTO_HOME_POSIX="$E2E_DIR/.proto-home"
  fi
fi

if [[ "$E2E_OS" == "windows" ]]; then
  # Mixed (forward-slash) form: accepted by Windows APIs and safe in bash.
  PROTO_HOME="$(cygpath -m "$_PROTO_HOME_POSIX")"
else
  PROTO_HOME="$_PROTO_HOME_POSIX"
fi

export PROTO_HOME
# PATH must use POSIX form (e.g. /c/Users/...) on Git Bash. Windows-mixed form
# (C:/Users/...) breaks bash's PATH splitting because the `C:` colon collides
# with the `:` PATH separator. MSYS translates POSIX PATH entries to Windows
# form when invoking native .exe binaries, so child processes still get the
# right paths.
export PATH="$_PROTO_HOME_POSIX/shims:$_PROTO_HOME_POSIX/bin:$PATH"

# Stable locale across runners (stderr matching shouldn't depend on it)
export LANG="${LANG:-C.UTF-8}"
export LC_ALL="${LC_ALL:-C.UTF-8}"

# proto detects AI agents and switches its output to NDJSON, which breaks every
# assertion in the suite — `just test-e2e` run from inside an agent fails on
# output no human ever sees. Neutralize the detection (an empty value counts as
# not set) and pin the reporter, the same way the Rust tests do in
# crates/core/src/test_utils.rs.
for _ai_var in \
  AI_AGENT ANTIGRAVITY_AGENT AUGMENT_AGENT CLAUDECODE CLAUDE_CODE \
  CLAUDE_CODE_IS_COWORK CODEX_CI CODEX_SANDBOX CODEX_THREAD_ID \
  COPILOT_ALLOW_ALL COPILOT_CLI COPILOT_GITHUB_TOKEN COPILOT_MODEL \
  CURSOR_AGENT CURSOR_EXTENSION_HOST_ROLE CURSOR_TRACE_ID GEMINI_CLI \
  OPENCODE OPENCODE_CLIENT REPL_ID; do
  export "$_ai_var="
done
unset _ai_var

export PROTO_JSON=false
export PROTO_REPORTER=text

# Match existing CI diagnostics
export PROTO_DEBUG_COMMAND=1
# export PROTO_DEBUG_SHIM=1 This breaks child processes!
export PROTO_DEBUG_WASM=1
export RUST_BACKTRACE=1
