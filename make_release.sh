#!/bin/bash
old_version=$1
new_version=$2
echo "Version $1 -> $2"
find . -name "*.sh" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;
find . -name "*.yaml" -exec grep -Hn ":$old_version" '{}' \; -exec sed -i "s/:$old_version/:$new_version/g" {} \;
