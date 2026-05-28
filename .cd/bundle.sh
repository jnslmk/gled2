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
    echo "Setting up macOS code signing keychain"

    KEYCHAIN_NAME="ci-signing.keychain"
    KEYCHAIN_PASSWORD="ci-$(date +%s)"

    # Create a fresh build keychain and add it to the search list alongside the login keychain
    security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
    security list-keychains -d user -s "$KEYCHAIN_NAME" $(security list-keychains -d user | tr -d '"')
    security default-keychain -s "$KEYCHAIN_NAME"
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
    # Prevent the keychain from auto-locking during the build
    security set-keychain-settings -t 3600 "$KEYCHAIN_NAME"

    # Import the certificate and grant codesign access to the private key.
    # set-key-partition-list is the key fix for errSecInternalComponent in CI.
    echo "$APPLE_CERTIFICATE" | base64 --decode > /tmp/gled-cert.p12
    security import /tmp/gled-cert.p12 -k "$KEYCHAIN_NAME" -P "$APPLE_CERTIFICATE_PASSWORD" \
        -T /usr/bin/codesign -T /usr/bin/security -T /usr/bin/productbuild
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
    rm -f /tmp/gled-cert.p12

    # Unset these so cargo-packager uses the identity already in the keychain
    # instead of creating its own cargo-packager.keychain (which lacks the partition list).
    unset APPLE_CERTIFICATE
    unset APPLE_CERTIFICATE_PASSWORD

    echo "Building Darwin dmg"
    rm -f *.dmg
    cargo build --target aarch64-apple-darwin --release
    strip target/aarch64-apple-darwin/release/gled
    cargo packager --target aarch64-apple-darwin --release -o . -f dmg --verbose

    # Clean up the build keychain
    security delete-keychain "$KEYCHAIN_NAME"
fi
