#!/bin/bash

# Input image
INPUT="../assets/logo.png"
IMAGE_NAME="org.squidowl.halloy.png"

# Sizes to generate
SIZES=(16 24 32 48 64 96 128 256 512)

# Check if input file exists
if [ ! -f "$INPUT" ]; then
    echo "Error: $INPUT not found!"
    exit 1
fi

# Loop over sizes
for SIZE in "${SIZES[@]}"; do
    TARGET_DIR="../assets/linux/icons/hicolor/${SIZE}x${SIZE}/apps"
    mkdir -p "$TARGET_DIR"
    sips -z "$SIZE" "$SIZE" "$INPUT" --out "$TARGET_DIR/$IMAGE_NAME" >/dev/null
    echo "Created: $TARGET_DIR/$IMAGE_NAME"
done

echo "✅ All linux icons resized and saved."
