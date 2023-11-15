#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) \{
    $DebugPreference = 'Continue'
}

{{ if alt_bin }}
& proto.exe run {bin} --alt "{alt_bin}" -- {before_args} $args {after_args}
{{ else }}
& proto.exe run {bin} -- {before_args} $args {after_args}
{{ endif }}

