#!/usr/bin/env bash
# requires: 18-install-moon
set -euo pipefail
source "$(dirname "$0")/../lib/env.sh"
source "$(dirname "$0")/../lib/assert.sh"

metrics_file="$PROTO_HOME/otel-metrics.txt"
rm -f "$metrics_file"

PROTO_TEST_OTEL_METRICS_FILE="$metrics_file" proto install moon 2.2 --force

if [[ -f "$metrics_file" ]]; then
  fail "OTEL metrics were recorded without --otel"
fi

PROTO_TEST_OTEL_METRICS_FILE="$metrics_file" proto --otel install moon 2.2 --force

assert_file "$metrics_file"

metrics="$(cat "$metrics_file")"
assert_contains "$metrics" "proto.tool.install.step.attempts"
assert_contains "$metrics" "proto.tool.install.duration"
