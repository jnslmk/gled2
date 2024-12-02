#!/bin/bash

set -e

rm -rf bin
mkdir -p bin

cargo install cargo-edit --locked
cargo install cargo-packager --locked

if [[ $(uname) == "Linux" ]]; then
    cargo install cargo-generate-rpm --locked
    cargo install cargo-xwin --locked
    cargo set-version ${CI_COMMIT_TAG_VERSION}

    # RPM
    cargo build --release
    strip target/release/gled
    cargo generate-rpm -o bin/gled-${CI_COMMIT_TAG_VERSION}.x86_64.rpm

    # Windows exe
    cargo-xwin build --target x86_64-pc-windows-msvc --release
    cargo packager --target x86_64-pc-windows-msvc --release -o bin -f nsis
fi

# Macos dmg
if [[ $(uname) == "Darwin" ]]; then
    cargo build --release
    cargo packager --release -o bin -f dmg
fi
