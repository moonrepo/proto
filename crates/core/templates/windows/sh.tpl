{% import "macros.tpl" as macros %}

#!/bin/sh
{{ macros::exec(args='"$@"') }}
