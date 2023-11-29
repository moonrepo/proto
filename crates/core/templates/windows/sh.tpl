{% import "macros.tpl" as macros %}

#!/bin/sh

{{ macros::cmd(args='"$@"') }}
