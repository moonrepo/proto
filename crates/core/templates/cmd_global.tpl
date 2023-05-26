{{ if alt_bin }}
proto.exe run {parent_name} --bin "{alt_bin}" -- %*
{{ else }}
proto.exe run {name} -- %*
{{ endif }}
