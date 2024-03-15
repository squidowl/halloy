#!/bin/bash
set -x
cd $(git rev-parse --show-toplevel)/assets

src=logo.png

conv_opts="-colors 256 -background none -density 300"

function icon_from_logo() {
  dims=$1

  convert $conv_opts -resize "!$dims" "$src" "linux/icons/hicolor/$dims/apps/org.squidowl.halloy.png"
}

# the linux icons
icon_from_logo "16x16"
icon_from_logo "24x24"
icon_from_logo "32x32"
icon_from_logo "48x48"
icon_from_logo "64x64"
icon_from_logo "96x96"
icon_from_logo "128x128"
icon_from_logo "256x256"
icon_from_logo "512x512"
