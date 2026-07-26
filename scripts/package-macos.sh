#!/bin/bash

PROFILE="packaging"
RELEASE_DIR="target/$PROFILE"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Halloy.app"
VERSION=$(grep -q '\..*\.' VERSION && cat VERSION || echo "$(cat VERSION).0")
NIGHTLY=$(cat NIGHTLY)
if [ "$VERSION" = "$NIGHTLY" ]; then
  DMG_NAME="halloy-nightly.dmg"
else
  DMG_NAME="halloy.dmg"
fi
DMG_DIR="$RELEASE_DIR"

# package dmg
echo "Packing disk image..."
# Remove disk images restored from caches created before DMG output was moved
# outside the source directory.
rm -f "$APP_DIR/halloy.dmg" "$APP_DIR/halloy-nightly.dmg"
ln -sf /Applications "$APP_DIR/Applications"
hdiutil create "$DMG_DIR/$DMG_NAME" -volname "Halloy" -fs HFS+ -srcfolder "$APP_DIR" -ov -format UDZO
echo "Packed '$APP_NAME' in '$DMG_DIR/$DMG_NAME'"
