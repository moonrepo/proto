{% import "macros.tpl" as macros %}

#!/usr/bin/env pwsh
$ErrorActionPreference = 'Stop'

if (Test-Path env:PROTO_DEBUG) \{
    $DebugPreference = 'Continue'
}

$ret = 0

if ($MyInvocation.ExpectingInput) {
    $input | & {{ macros::cmd(args="$args") }}
} else {
    & {{ macros::cmd(args="$args") }}
}

$ret = $LASTEXITCODE
exit $ret
