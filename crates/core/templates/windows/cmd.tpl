@echo off
@setLocal

set "ErrorActionPreference=Stop"

if defined PROTO_DEBUG (
    set "DebugPreference=Continue"
)

{{ if alt_bin }}
endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & proto.exe run {bin} --alt "{alt_bin}" -- {before_args} %* {after_args}
{{ else }}
endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & proto.exe run {bin} -- {before_args} %* {after_args}
{{ endif }}
