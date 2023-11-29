#!/usr/bin/env bash
set -e
[ -n "$PROTO_DEBUG" ] && set -x

exec proto run {{ bin }} {% if alt_bin %} --alt "{{ alt_bin }}" {% endif %}-- {{ before_args }} "$@" {{ after_args }}
