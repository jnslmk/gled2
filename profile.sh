#!/bin/bash

echo "We need to enable system perf events, so please enter your user password:"
echo '-1' | sudo tee /proc/sys/kernel/perf_event_paranoid

echo "Installing samply"
cargo install --locked samply

echo "Building gled with profiling profile"
cargo build --profile profiling

echo "Running gled with samply"
samply record ./target/profiling/gled
