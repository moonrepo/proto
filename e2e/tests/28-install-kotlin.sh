#!/usr/bin/env bash
# requires: 24-install-java-jdk
set -euo pipefail
source "$(dirname "$0")/../lib/utils.sh"

# Kotlin's launchers require a Java runtime, so point them at the JDK
# installed by a previous test, instead of whatever the host provides.
# Strip either separator, as the path is Windows-shaped in Git Bash.
jdk_bin="$(proto bin jdk --dir exes)"
export JAVA_HOME="${jdk_bin%[\\/]*}"

install_tool kotlin 2.4.20 -version
