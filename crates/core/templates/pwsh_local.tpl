{{ if tool_dir }}
[Environment]::SetEnvironmentVariable('PROTO_{bin | uppercase}_DIR', '{tool_dir}', 'Process')
{{ endif }}

{{ if tool_version }}
[Environment]::SetEnvironmentVariable('PROTO_{bin | uppercase}_VERSION', '{tool_version}', 'Process')
{{ endif }}

{{ if parent_bin }}
if (Test-Path env:PROTO_{parent_bin | uppercase}_BIN) \{
    $parent = $Env:PROTO_{parent_bin | uppercase}_BIN
} else \{
    $parent = "{parent_bin}"
}

& "$parent" "{bin_path}" $args
{{ else }}

& "{bin_path}" $args
{{ endif }}

exit $LASTEXITCODE
