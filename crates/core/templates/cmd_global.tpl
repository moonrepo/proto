{{ if alt_bin }}
proto.exe run {parent_bin} --bin "{alt_bin}" -- %*
{{ else }}
proto.exe run {bin} -- %*
{{ endif }}
