#!/bin/bash

set -e
cd $(dirname "${BASH_SOURCE[0]}")

paths=$(cat paths/$1.txt)

IFS_BAK=$IFS
IFS=$'\n'

for path in $paths; do
    if [[ "$path" != "" ]]; then
        ./check.sh jsons/$1.json "$path"
    fi
done 

IFS=$IFS_BAK
IFS_BAK=