#!/bin/bash

set -e
cd $(dirname "${BASH_SOURCE[0]}")

rm -rf go-cli/go-cli rust-cli/target
