{{ if tool_dir }}
export PROTO_{bin | uppercase}_DIR="{tool_dir}"
{{ endif }}

{{ if tool_version }}
export PROTO_{bin | uppercase}_VERSION="{tool_version}"
{{ endif }}

{{ if parent_bin }}
parent="$\{PROTO_{parent_bin | uppercase}_BIN:-{parent_bin}}"

exec "$parent" "{bin_path}" "$@"
{{ else }}

exec "{bin_path}" "$@"
{{ endif }}
