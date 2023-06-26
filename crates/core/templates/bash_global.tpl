{{ if alt_bin }}
exec proto run {parent_bin} --bin "{alt_bin}" -- "$@"
{{ else }}
exec proto run {bin} -- "$@"
{{ endif }}
