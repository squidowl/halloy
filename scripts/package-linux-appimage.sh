#!/bin/bash -e

ARCH="x86_64"
TARGET="halloy"
VERSION=$(cat VERSION)
PROFILE="release"
ASSETS_DIR="assets/linux"
RELEASE_DIR="target/$PROFILE"
APPDIR="$RELEASE_DIR/AppDir"
APPIMAGE_NAME="$TARGET-$VERSION-$ARCH.AppImage"
APPIMAGE_PATH="$RELEASE_DIR/$APPIMAGE_NAME"

appimage_name() {
  echo $APPIMAGE_NAME
}

appimage_path() {
  echo $APPIMAGE_PATH
}

package() {
  # Create necessary directories
  mkdir -p "$APPDIR/usr/bin"
  mkdir -p "$APPDIR/usr/share/applications"
  mkdir -p "$APPDIR/usr/share/metainfo/"

  # Copy desktop stuff
  cp assets/linux/org.squidowl.halloy.desktop "$APPDIR/usr/share/applications/"
  cp assets/linux/org.squidowl.halloy.desktop "$APPDIR/usr/share/metainfo/"
  cp assets/linux/org.squidowl.halloy.appdata.xml "$APPDIR/usr/share/metainfo/"

  cp assets/linux/org.squidowl.halloy.desktop "$APPDIR/"

  cp -r assets/linux/icons "$APPDIR/usr/share/"
  ln -rs "$APPDIR/usr/share/icons/hicolor/256x256/apps/org.squidowl.halloy.png" "$APPDIR/org.squidowl.halloy.png"

  # Build the Rust binary
  cargo install --profile $PROFILE --path . --root "$APPDIR/usr/"

  # Create our AppRun file
  ln -rs "$APPDIR/usr/bin/halloy" "$APPDIR/AppRun"

  ./appimagetool-x86_64.AppImage "$APPDIR" "$APPIMAGE_PATH"
}

case "$1" in
  "package") package;;
  "appimage_name") appimage_name;;
  "appimage_path") appimage_path;;
  *)
    echo "available commands: package, appimage_name, appimage_path"
    ;;
esac
