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

  install -Dm755 $BINARY -t $ARCHIVE_DIR/bin
  install -Dm644 $ASSETS_DIR/org.squidowl.halloy.appdata.xml -t $ARCHIVE_DIR/share/metainfo
  install -Dm644 $ASSETS_DIR/org.squidowl.halloy.desktop -t $ARCHIVE_DIR/share/applications
  cp -r $ASSETS_DIR/icons $ARCHIVE_DIR/share/

  tar czvf $ARCHIVE_PATH -C $ARCHIVE_DIR .

  echo "Packaged archive: $ARCHIVE_PATH"
}

case "$1" in
  "package") package;;
  "archive_name") archive_name;;
  "archive_path") archive_path;;
  *)
    echo "available commands: package, archive_name, archive_path"
    ;;
esac
