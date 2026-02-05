#! /usr/bin/env -S bash -e

TARGET="halloy"
ASSETS_DIR="assets"
PROFILE="packaging"
RELEASE_DIR="target/$PROFILE"
APP_NAME="Halloy.app"
APP_TEMPLATE="$ASSETS_DIR/macos/$APP_NAME"
APP_TEMPLATE_PLIST="$APP_TEMPLATE/Contents/Info.plist"
APP_DIR="$RELEASE_DIR/macos"
APP_BINARY="$RELEASE_DIR/$TARGET"
APP_BINARY_DIR="$APP_DIR/$APP_NAME/Contents/MacOS"
APP_EXTRAS_DIR="$APP_DIR/$APP_NAME/Contents/Resources"

DMG_NAME="halloy.dmg"
DMG_DIR="$RELEASE_DIR/macos"

VERSION=$(cat VERSION)
BUILD=$(git describe --always --dirty --exclude='*')

# update version and build
sed -i '' -e "s/{{ VERSION }}/$VERSION/g" "$APP_TEMPLATE_PLIST"
sed -i '' -e "s/{{ BUILD }}/$BUILD/g" "$APP_TEMPLATE_PLIST"

# build binary
export MACOSX_DEPLOYMENT_TARGET="11.0"
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
cargo build --profile $PROFILE --locked --target=x86_64-apple-darwin
cargo build --profile $PROFILE --locked --target=aarch64-apple-darwin
lipo "target/x86_64-apple-darwin/$PROFILE/$TARGET" "target/aarch64-apple-darwin/$PROFILE/$TARGET" -create -output "$APP_BINARY"

# build app
mkdir -p "$APP_BINARY_DIR"
mkdir -p "$APP_EXTRAS_DIR"
cp -fRp "$APP_TEMPLATE" "$APP_DIR"
cp -fp "$APP_BINARY" "$APP_BINARY_DIR"
touch -r "$APP_BINARY" "$APP_DIR/$APP_NAME"
echo "Created '$APP_NAME' in '$APP_DIR'"
