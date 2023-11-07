{{ if alt_bin }}
exec proto run {bin} --alt "{alt_bin}" -- {before_args} "$@" {after_args}
{{ else }}
exec proto run {bin} -- {before_args} "$@" {after_args}
{{ endif }}
