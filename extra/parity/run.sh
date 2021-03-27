#!/bin/bash

set -e
cd $(dirname "${BASH_SOURCE[0]}")

lines=$(ls paths/*.txt)

for line in $lines; do
    ./many.sh "$(basename -s '.txt' $line)"
done
