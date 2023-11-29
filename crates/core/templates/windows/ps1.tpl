{% import "macros.tpl" as macros %}

#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) {
    $DebugPreference = 'Continue'
    Write-Output "Running with {{ bin }}.ps1 shim"
}

$ret = 0

if ($MyInvocation.ExpectingInput) {
    $input | & {{ macros::exec(args="$args") }}
} else {
    & {{ macros::exec(args="$args") }}
}

$ret = $LASTEXITCODE
exit $ret
