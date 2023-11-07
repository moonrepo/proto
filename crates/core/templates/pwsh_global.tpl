{{ if alt_bin }}
& proto.exe run {bin} --alt "{alt_bin}" -- {before_args} $args {after_args}
{{ else }}
& proto.exe run {bin} -- {before_args} $args {after_args}
{{ endif }}
