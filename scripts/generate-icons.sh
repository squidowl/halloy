#!/bin/bash
set -x
cd $(git rev-parse --show-toplevel)/assets

src=logo.png

conv_opts="-colors 256 -background none -density 300"

# the linux icon
for size in "16" "24" "32" "48" "64" "96" "128" "256" "512"; do
  target="linux/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$target"
  convert $conv_opts -resize "!${size}x${size}" "$src" "$target/org.squidowl.halloy.png"
done
