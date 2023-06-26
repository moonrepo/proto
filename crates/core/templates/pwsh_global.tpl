{{ if alt_bin }}
& proto.exe run {parent_bin} --bin "{alt_bin}" -- $args
{{ else }}
& proto.exe run {bin} -- $args
{{ endif }}
