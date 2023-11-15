@echo off
setlocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
)

{{ if alt_bin }}
proto.exe run {bin} --alt "{alt_bin}" -- {before_args} %* {after_args}
{{ else }}
proto.exe run {bin} -- {before_args} %* {after_args}
{{ endif }}
