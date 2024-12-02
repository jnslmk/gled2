#!/bin/bash

set -e

cargo install cargo-edit cargo-packager cargo-generate-rpm cargo-xwin --locked

echo "Setting version to ${CI_COMMIT_TAG}"
cargo set-version ${CI_COMMIT_TAG}

# RPM
cargo build --release
strip target/release/gled
cargo generate-rpm -o gled-${CI_COMMIT_TAG}.x86_64.rpm

# Windows exe
cargo-xwin build --target x86_64-pc-windows-msvc --release
cargo packager --target x86_64-pc-windows-msvc --release -o . -f nsis
