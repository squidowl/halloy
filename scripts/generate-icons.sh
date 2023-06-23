#!/bin/bash
set -x
cd $(git rev-parse --show-toplevel)/assets

src=logo.png

conv_opts="-colors 256 -background none -density 300"

# the linux icon
convert $conv_opts -resize "!128x128" "$src" "logo_128px.png"
