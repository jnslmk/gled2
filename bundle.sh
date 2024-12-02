#!/bin/bash

set -e

echo "test" >gled-${CI_COMMIT_TAG}.x86_64.rpm
echo "test" >gled_${CI_COMMIT_TAG}_x64-setup.exe
exit 0

cargo install cargo-edit --locked
cargo install cargo-packager --locked

if [[ $(uname) == "Linux" ]]; then
    cargo install cargo-generate-rpm --locked
    cargo install cargo-xwin --locked
    cargo set-version ${CI_COMMIT_TAG_VERSION}

    # RPM
    cargo build --release
    strip target/release/gled
    cargo generate-rpm -o gled-${CI_COMMIT_TAG_VERSION}.x86_64.rpm

    # Windows exe
    cargo-xwin build --target x86_64-pc-windows-msvc --release
    cargo packager --target x86_64-pc-windows-msvc --release -o . -f nsis
fi

# Macos dmg
if [[ $(uname) == "Darwin" ]]; then
    cargo build --release
    cargo packager --release -o . -f dmg
fi
