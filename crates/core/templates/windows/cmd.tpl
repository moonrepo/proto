{% import "macros.tpl" as macros %}

@echo off
setLocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
    echo "Running with {{ bin }}.cmd"
)

{# This hack removes the "Terminate Batch Job" message #}
endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & {{ macros::exec(args="%*") }}
