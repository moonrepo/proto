{% import "macros.tpl" as macros %}

@echo off
setlocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
    echo "Running with {{ bin }}.cmd shim"
)

{# This hack removes the "Terminate Batch Job" message #}
endlocal & goto #_undefined_# 2>NUL || title %COMSPEC% & {{ macros::exec(args="%*") }}
