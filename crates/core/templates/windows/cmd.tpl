{% import "macros.tpl" as macros %}

@echo off
setlocal

if defined PROTO_DEBUG (
    echo "Running with {{ bin }}.cmd shim"
)

{# This hack removes the "Terminate Batch Job" message #}
endlocal & goto #_undefined_# 2>NUL || title %COMSPEC% & {{ macros::exec(args="%*") }}
