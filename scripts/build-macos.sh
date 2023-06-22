#!/bin/bash

TARGET="halloy"
ASSETS_DIR="assets"
RELEASE_DIR="target/release"
APP_NAME="Halloy.app"
APP_TEMPLATE="$ASSETS_DIR/macos/$APP_NAME"
APP_DIR="$RELEASE_DIR/macos"
APP_BINARY="$RELEASE_DIR/$TARGET"
APP_BINARY_DIR="$APP_DIR/$APP_NAME/Contents/MacOS"
APP_EXTRAS_DIR="$APP_DIR/$APP_NAME/Contents/Resources"
APP_COMPLETIONS_DIR="$APP_EXTRAS_DIR/completions"

DMG_NAME="halloy.dmg"
DMG_DIR="$RELEASE_DIR/macos"

binary() {
    export MACOSX_DEPLOYMENT_TARGET="11.0"
    cargo build --release --target=x86_64-apple-darwin
    cargo build --release --target=aarch64-apple-darwin
    lipo "target/x86_64-apple-darwin/release/$TARGET" "target/aarch64-apple-darwin/release/$TARGET" -create -output "$APP_BINARY"
}

app() {
    mkdir -p "$APP_BINARY_DIR"
    mkdir -p "$APP_EXTRAS_DIR"
    mkdir -p "$APP_COMPLETIONS_DIR"
    cp -fRp "$APP_TEMPLATE" "$APP_DIR"
    cp -fp "$APP_BINARY" "$APP_BINARY_DIR"
    touch -r "$APP_BINARY" "$APP_DIR/$APP_NAME"
    echo "Created '$APP_NAME' in '$APP_DIR'"
}

dmg() {
    echo "Packing disk image..."
    ln -sf /Applications "$DMG_DIR/Applications"
    hdiutil create "$DMG_DIR/$DMG_NAME" -volname "Halloy" -fs HFS+ -srcfolder "$APP_DIR" -ov -format UDZO
    echo "Packed '$APP_NAME' in '$APP_DIR'"
}


clean() {
    cargo clean
}

case "$1" in
    binary)
        binary
        ;;
    app)
        app
        ;;
    dmg)
        app
        dmg
        ;;
    clean)
        clean
        ;;
    *)
        echo "  binary      Build the $TARGET binary"
        echo "  app         Create the $APP_NAME application bundle"
        echo "  dmg         Create the $DMG_NAME disk image"
        echo "  clean       Clean the project"
        ;;
esac
