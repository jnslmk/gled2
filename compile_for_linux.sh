cargo build --release
strip target/release/gled
gzexe --best target/release/gled