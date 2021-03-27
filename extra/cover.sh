#!/bin/bash

set -e

echo 'Copying project files'
scp -rCq Cargo.toml src testfiles josh@lab:gjson-rs
ssh josh@lab <<'ENDSSH'
    cd gjson-rs
    cargo tarpaulin --ignore-tests -o Html
ENDSSH
scp josh@lab:gjson-rs/tarpaulin-report.html /tmp/coverage.html

echo "details: file:///tmp/coverage.html"
