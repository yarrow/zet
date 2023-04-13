#!/bin/sh
set -e
cargo clippy --tests --no-deps
cargo clippy --profile=test --no-deps
cargo test
