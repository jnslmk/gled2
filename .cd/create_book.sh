#!/bin/bash

set -e

curl -L "https://github.com/rust-lang/mdBook/releases/download/v0.4.43/mdbook-v0.4.43-x86_64-unknown-linux-gnu.tar.gz" -o mdbook.tar.gz
tar xvfz mdbook.tar.gz
./mdbook build ./book -d ../public
