{{ if alt_bin }}
exec proto run {parent_bin} --bin "{alt_bin}" -- {before_args} "$@" {after_args}
{{ else }}
exec proto run {bin} -- {before_args} "$@" {after_args}
{{ endif }}
