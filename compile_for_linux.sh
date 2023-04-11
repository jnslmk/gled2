set -e

cargo build --release
strip target/release/gled
gzexe target/release/gled