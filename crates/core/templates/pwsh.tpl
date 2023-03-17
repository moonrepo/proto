{{ if install_dir }}
[Environment]::SetEnvironmentVariable('PROTO_{name | uppercase}_DIR', '{install_dir}', 'Process')
{{ endif }}

{{ if version }}
[Environment]::SetEnvironmentVariable('PROTO_{name | uppercase}_VERSION', '{version}', 'Process')
{{ endif }}

{{ if parent_name }}
if (Test-Path env:PROTO_{parent_name | uppercase}_BIN) \{
    $parent = $Env:PROTO_{parent_name | uppercase}_BIN
} else \{
    $parent = "{parent_name}.exe"
}

& "$parent" "{bin_path}" $args
{{ else }}

& "{bin_path}" $args
{{ endif }}

exit $LASTEXITCODE
