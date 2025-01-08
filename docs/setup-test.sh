#!/usr/bin/env bash

# This script loosely resembles https://github.com/moonrepo/moon/blob/master/website/static/install/proto.sh
# so that we can test stdin and other things correctly

set -eo pipefail

echo "> Gathering args"
echo "  $@"

echo ""
echo "> Changing PATH"
echo "  Before=$PATH"

newPath="$PATH"

shimsDir="$HOME/.proto/shims:"
newPath=${newPath/"$shimsDir"/""}

binsDir="$HOME/.proto/bin:"
newPath=${newPath/"$binsDir"/""}

echo ""
echo "  After=$newPath"

export PATH="$newPath"

echo ""
echo "> Running setup"

exec cargo run -- setup --log trace
