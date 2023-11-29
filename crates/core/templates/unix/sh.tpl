#!/usr/bin/env bash
set -e

if [ -n "$PROTO_DEBUG" ]; then
    set -x
    echo "Running with {{ bin }}.sh shim"
fi

exec proto run {{ bin }} {% if alt_bin %}--alt "{{ alt_bin }}" {% endif %}-- {{ before_args }} "$@" {{ after_args }}
