#!/bin/bash

echo "Installing puffin_viewer"
cargo install puffin_viewer

echo "Running gled with profiling profile"
cargo run --profile profiling --features profiling
