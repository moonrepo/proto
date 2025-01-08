# This script loosely resembles https://github.com/moonrepo/moon/blob/master/website/static/install/proto.ps1
# so that we can test stdin and other things correctly

Write-Output "> Gathering args"
Write-Output "  $Args"

Write-Output ""
Write-Output "> Changing PATH"
Write-Output "  Before=$env:PATH"

$newPath = "$env:PATH"

$shimsDir = "${HOME}\.proto\shims;"
$newPath = $newPath.Replace("$shimsDir", "");

$binsDir = "${HOME}\.proto\bin;"
$newPath = $newPath.Replace("$binsDir", "");

Write-Output ""
Write-Output "  After=$newPath"

$env:Path = "$newPath"

Write-Output ""
Write-Output "> Running setup"

cargo run -- setup --log trace
