#!/bin/bash

INPUT="../assets/logo-macos.png"
ICONSET="icon.iconset"
OUTPUT="../assets/macos/Halloy.app/Contents/Resources/halloy.icns"

# Check input file
if [ ! -f "$INPUT" ]; then
  echo "❌ Error: '$INPUT' not found."
  exit 1
fi

# Create iconset folder
mkdir -p "$ICONSET"

echo "🔧 Generating iconset..."

# Resize
magick "$INPUT" -resize 16x16     -strip PNG32:"$ICONSET/icon_16x16.png"
magick "$INPUT" -resize 32x32     -strip PNG32:"$ICONSET/icon_16x16@2x.png"
magick "$INPUT" -resize 32x32     -strip PNG32:"$ICONSET/icon_32x32.png"
magick "$INPUT" -resize 64x64     -strip PNG32:"$ICONSET/icon_32x32@2x.png"
magick "$INPUT" -resize 128x128   -strip PNG32:"$ICONSET/icon_128x128.png"
magick "$INPUT" -resize 256x256   -strip PNG32:"$ICONSET/icon_128x128@2x.png"
magick "$INPUT" -resize 256x256   -strip PNG32:"$ICONSET/icon_256x256.png"
magick "$INPUT" -resize 512x512   -strip PNG32:"$ICONSET/icon_256x256@2x.png"
magick "$INPUT" -resize 512x512   -strip PNG32:"$ICONSET/icon_512x512.png"
magick "$INPUT" -resize 1024x1024 -strip PNG32:"$ICONSET/icon_512x512@2x.png"

# Create the .icns file
echo "📦 Creating .icns file..."
iconutil -c icns "$ICONSET" -o "$OUTPUT"

# Check if iconutil succeeded
if [ $? -eq 0 ]; then
  echo "✅ .icns file created at $OUTPUT"
else
  echo "❌ Error: Failed to create .icns file"
  exit 1
fi

# Clean up
rm -r "$ICONSET"

echo "🎉 Icon generation complete!"