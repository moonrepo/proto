#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) \{
    $DebugPreference = 'Continue'
}

& proto.exe run {name} -- $args