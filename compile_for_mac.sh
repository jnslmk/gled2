set -e

rm -rf /tmp/gled
mkdir /tmp/gled
cargo build --release --target x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin
cp -r assets/Gled.app /tmp/gled/
lipo -create -output /tmp/gled/Gled.app/Contents/MacOS/gled target/aarch64-apple-darwin/release/gled target/x86_64-apple-darwin/release/gled
brew install create-dmg
rm -f rw.Gled.dmg Gled.dmg
create-dmg \
    --volname "Gled Installer" \
    --volicon "Gled.app/Contents/Resources/icon.icns" \
    --window-pos 200 120 \
    --window-size 800 400 \
    --icon-size 100 \
    --icon "Gled.app" 200 190 \
    --hide-extension "Gled.app" \
    --app-drop-link 600 185 \
    "Gled.dmg" \
    "/tmp/gled/"
