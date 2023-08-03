@echo off
setlocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
)