cargo build --release --target x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin
lipo -create Gled.app/Contens/MacOS/gled target/aarch64-apple-darwin/release/gled target/x86_64-apple-darwin/release/gled
