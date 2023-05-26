{{ if alt_bin }}
call proto.exe run {parent_name} --bin "{alt_bin}" -- %*
{{ else }}
call proto.exe run {name} -- %*
{{ endif }}
