#!/bin/bash

ARCH="x86_64"
TARGET="halloy"
VERSION=$(cat VERSION)
PROFILE="release"
ASSETS_DIR="assets/linux"
RELEASE_DIR="target/$PROFILE"
BINARY="$RELEASE_DIR/$TARGET"
ARCHIVE_DIR="$RELEASE_DIR/archive"
ARCHIVE_NAME="$TARGET-$VERSION-$ARCH-linux.tar.gz"
ARCHIVE_PATH="$RELEASE_DIR/$ARCHIVE_NAME"

build() {
  cargo build --profile $PROFILE
}

archive_name() {
  echo $ARCHIVE_NAME
}

archive_path() {
  echo $ARCHIVE_PATH
}

package() {
  build

  install -D $ASSETS_DIR/* -t $ARCHIVE_DIR
  for ASSET_PATH in $ASSETS_DIR/icons/hicolor/*/apps/org.squidowl.halloy.png; do
    ASSET_ARCHIVE_PATH="$ARCHIVE_DIR/${ASSET_PATH#"$ASSETS_DIR"}"
    install -Dm644 $ASSET_PATH $ASSET_ARCHIVE_PATH
  done
  install -Dm755 $BINARY $ARCHIVE_DIR
  tar czvf $ARCHIVE_PATH -C $ARCHIVE_DIR .

  echo "Packaged archive: $ARCHIVE_PATH"
}

case "$1" in
  "package") package;;
  "archive_name") archive_name;;
  "archive_path") archive_path;;
  *)
    echo "avaiable commands: package, archive_name, archive_path"
    ;;
esac
