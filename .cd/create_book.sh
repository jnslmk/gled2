#!/bin/bash

set -e

curl "https://objects.githubusercontent.com/github-production-release-asset-2e65be/38655056/fa65099c-dba9-4076-8de0-19ccbdbe843f?X-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=releaseassetproduction%2F20241224%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20241224T090808Z&X-Amz-Expires=300&X-Amz-Signature=363ec9224a3daa745026afa5b57060f0a8269271b25fadef0a96cbab6dfc5cf3&X-Amz-SignedHeaders=host&response-content-disposition=attachment%3B%20filename%3Dmdbook-v0.4.43-x86_64-unknown-linux-gnu.tar.gz&response-content-type=application%2Foctet-stream" -o mdbook.tar.gz
tar xvfz mdbook.tar.gz
./mdbook build ./book -d ../public
