#!/bin/bash

# Input and output paths
INPUT="../assets/logo.png"
BASENAME="halloy"
TARGET_DIR="../assets/windows"
OUTPUT="$TARGET_DIR/$BASENAME.ico"

# Ensure input file exists
if [ ! -f "$INPUT" ]; then
  echo "❌ Error: File '$INPUT' not found."
  exit 1
fi

# Ensure target directory exists
mkdir -p "$TARGET_DIR"

# Create resized PNGs
echo "🔧 Resizing images..."
for SIZE in 512 256 128 64 48 32 16; do
  magick "$INPUT" -resize ${SIZE}x${SIZE} "${BASENAME}-${SIZE}.png"
done

# Create the .ico file from resized images
echo "🎯 Generating ICO file..."
magick \
  "${BASENAME}-512.png" \
  "${BASENAME}-256.png" \
  "${BASENAME}-128.png" \
  "${BASENAME}-64.png" \
  "${BASENAME}-48.png" \
  "${BASENAME}-32.png" \
  "${BASENAME}-16.png" \
  "$OUTPUT"

# Clean up intermediate PNGs
echo "🧹 Cleaning up temporary files..."
# rm "${BASENAME}"-{512,256,128,64,48,32,16}.png

echo "✅ ICO file created: $OUTPUT"
