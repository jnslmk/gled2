set -e

cargo build --release --target x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin
lipo -create -output Gled.app/Contens/MacOS/gled target/aarch64-apple-darwin/release/gled target/x86_64-apple-darwin/release/gled
rm -rf /tmp/gled
mkdir /tmp/gled
cp -r Gled.app /tmp/gled/
brew install create-dmg
create-dmg \
    --volname "Gled" \
    --volicon "Gled.app/Contents/Resources/icon.icns" \
    --window-pos 200 120 \
    --window-size 800 400 \
    --icon-size 100 \
    --icon "Gled.app" 200 190 \
    --hide-extension "Gled.app" \
    --app-drop-link 600 185 \
    "Gled.dmg" \
    "/tmp/gled/"