#!/bin/bash

set -e

cargo install cargo-edit --locked
VERSION=$(cargo set-version --bump minor 2>&1 | awk {'print $6'})

git commit Cargo.toml Cargo.lock -m "bump version"
git push

git tag $VERSION
git push --tag
