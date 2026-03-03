#!/bin/bash

set -e

cargo binstall cargo-edit
VERSION=$(cargo set-version --bump patch 2>&1 | awk {'print $6'})

git commit Cargo.toml Cargo.lock -m "bump version"
git push

git tag $VERSION
git push --tag
