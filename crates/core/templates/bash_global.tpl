{{ if alt_bin }}
exec proto run {name} --bin "{alt_bin}" -- "$@"
{{ else }}
exec proto run {name} -- "$@"
{{ endif }}
