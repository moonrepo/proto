{{ if tool_dir }}
export PROTO_{bin | uppercase}_DIR="{tool_dir}"
{{ endif }}

{{ if tool_version }}
export PROTO_{bin | uppercase}_VERSION="{tool_version}"
{{ endif }}

{{ if parent_bin }}
parent="$\{PROTO_{parent_bin | uppercase}_BIN:-{parent_bin}}"

exec "$parent" "{bin_path}" {before_args} $@ {after_args}
{{ else }}

exec "{bin_path}" {before_args} $@ {after_args}
{{ endif }}
