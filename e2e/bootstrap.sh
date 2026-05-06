#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib/env.sh"

# Create the proto store!
mkdir -p "$PROTO_HOME/bin"
mkdir -p "$PROTO_HOME/shims"

# Copy the built proto binary to the store. This is what tests will invoke via PATH.
# We don't want to rely on cargo's target dir structure in the tests,
# and this also simulates the real user experience better.
if [[ "$E2E_OS" == "windows" ]]; then
	cp "$REPO_ROOT/target/release/proto.exe" "$PROTO_HOME/bin"
	cp "$REPO_ROOT/target/release/proto-shim.exe" "$PROTO_HOME/bin"
else
	cp "$REPO_ROOT/target/release/proto" "$PROTO_HOME/bin"
	cp "$REPO_ROOT/target/release/proto-shim" "$PROTO_HOME/bin"
fi

# Print the directory contents for debugging. If the copy above failed, this will show it.
ls -lR "$PROTO_HOME"
