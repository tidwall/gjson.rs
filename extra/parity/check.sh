#!/bin/bash

# Checks if gjson results match exactly for Rust and Go, and that the results
# are valid utf8.
# example: ./check.sh ../testfiles/twitter.json statuses.#

set -e
cd $(dirname "${BASH_SOURCE[0]}")

if [[ "$1" == "" || "$2" == "" ]]; then
    echo "usage: $0 json-file gjson-path"
    exit 1
fi

if [ -t 0 ] && [ -t 1  ]; then
    printf "parity %s \e[2m%s\e[0m ... " "$(basename "$1")" "$2"
else
    printf "parity %s %s ... " "$(basename "$1")" "$2"
fi 

# Get the Rust result
if [[ "$RUST_RELEASE" == "" ]]; then
    RUST_TARGET="debug"
else 
    RUST_TARGET="release"
    RUST_FLAGS="--release"
fi

cd rust-cli
if [[ ! -f target/$RUST_TARGET/rust-cli ]]; then
    cargo build --quiet $RUST_FLAGS
fi
cd ..
RUST_RESULT="$(rust-cli/target/$RUST_TARGET/rust-cli "$1" "$2")"

# Get the Go result
cd go-cli
if [[ ! -f go-cli ]]; then
    go mod tidy
    go build -o go-cli main.go > /dev/null
fi
cd ..
GO_RESULT="$(go-cli/go-cli "$1" "$2")"

# Compare the Go and Rust results
if [[ "$RUST_RESULT" != "$GO_RESULT" ]]; then
    if [ -t 0 ] && [ -t 1  ]; then
        printf "\e[31mfail\e[0m\n"
    else 
        printf "fail\n"
    fi
    exit 1
fi

# Return the results to the caller
if [[ "$RUST_RESULT" == "" ]]; then 
    MISSING="(non-existent)"
fi
if [ -t 0 ] && [ -t 1  ]; then
    printf "\e[32mok\e[0m %s\n" "$MISSING"
else
    printf "ok %s\n" "$MISSING"
fi
