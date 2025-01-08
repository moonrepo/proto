#!/usr/bin/env bash
set -eo pipefail

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

echo "> Running setup"

echo "" | cargo run -- setup --log trace
