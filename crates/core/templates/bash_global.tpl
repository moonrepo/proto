{{ if alt_bin }}
exec proto run {parent_name} --bin "{alt_bin}" -- "$@"
{{ else }}
exec proto run {name} -- "$@"
{{ endif }}
