{{ if bin_path }}
exec proto run {bin} --bin "{bin_path}" -- {before_args} "$@" {after_args}
{{ else }}
exec proto run {bin} -- {before_args} "$@" {after_args}
{{ endif }}
