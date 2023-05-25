{{ if alt_bin }}
& proto.exe run {parent_name} --bin "{alt_bin}" -- $args
{{ else }}
& proto.exe run {name} -- $args
{{ endif }}
