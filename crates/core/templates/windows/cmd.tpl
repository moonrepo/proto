{% import "macros.tpl" as macros %}

@echo off
setLocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
)

endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & {{ macros::cmd(args="%*") }}
