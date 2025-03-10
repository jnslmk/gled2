#!/bin/bash
set -e

cargo install cargo-packager --locked

if [[ $(uname) == "Linux" ]]; then
    cargo install cargo-generate-rpm cargo-xwin --locked

    echo "Publishing to crates.io"
    cargo publish

    echo "Building Linux rpm"
    cargo build --release
    strip target/release/gled
    cargo generate-rpm -o gled-${CI_COMMIT_TAG}.x86_64.rpm

    echo "Building Windows exe"
    cargo-xwin build --target x86_64-pc-windows-msvc --release
    cargo packager --target x86_64-pc-windows-msvc --release -o . -f nsis
fi

if [[ $(uname) == "Darwin" ]]; then
    echo "Building Darwin dmg"
    rm -f *.dmg
    cargo build --target aarch64-apple-darwin --release
    strip target/aarch64-apple-darwin/release/gled
    cargo packager --target aarch64-apple-darwin --release -o . -f dmg --verbose
fi
