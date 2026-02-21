#!/bin/bash

if [ ! -f ./VERSION ]; then
    echo "VERSION file not found"
    exit 1
fi
VERSION=$(cat ./VERSION)

# First we make sure with have appimagetool-x86_64.AppImage
echo "Using halloy version: $VERSION for AppImage build"
if [ ! -f ./assets/linux/appimagetool-x86_64.AppImage ]; then
  echo "Downloading appimagetool-x86_64.AppImage..."
  curl -L 'https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage' -o ./assets/linux/appimagetool-x86_64.AppImage
  chmod +x ./assets/linux/appimagetool-x86_64.AppImage
fi

script_dir=$(dirname -- "$(realpath -- "${BASH_SOURCE[0]}")")
git_root_dir=$(git -C "$script_dir" rev-parse --show-toplevel)
build_dir="/tmp/halloy.AppDir"
appimage_output_dir="$git_root_dir/target/release/linux-appimage"
appimage_name="halloy-${VERSION}-x86_64.AppImage"

# Clean up any previous build
echo "Cleaning up previous build directory..."
rm -rf "$build_dir"

# Create necessary directories
echo "Creating build directories..."
mkdir -p "$appimage_output_dir"
mkdir -p "$build_dir/usr/bin"
mkdir -p "$build_dir/usr/share/applications"
mkdir -p "$build_dir/usr/share/metainfo/"

# Copy desktop stuff
echo "Copying desktop files..."
cp "$git_root_dir/assets/linux/org.squidowl.halloy.desktop" "$build_dir/usr/share/applications/"
cp "$git_root_dir/assets/linux/org.squidowl.halloy.desktop" "$build_dir/usr/share/metainfo/"
cp "$git_root_dir/assets/linux/org.squidowl.halloy.appdata.xml" "$build_dir/usr/share/metainfo/"

cp "$git_root_dir/assets/linux/org.squidowl.halloy.desktop" "$build_dir/"
cp "$git_root_dir/assets/linux/icons/hicolor/256x256/apps/org.squidowl.halloy.png" "$build_dir/"

# Create our AppRun file
echo "Creating AppRun file..."
cat << 'EOF' > "$build_dir/AppRun"
#!/bin/sh

cd "$(dirname "$0")" || exit 1

# Launch the application with specific logging settings
exec ./usr/bin/halloy
EOF

chmod +x "$build_dir/AppRun"

# Build the Rust binary
echo "Building Rust binary..."

# Check cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "cargo could not be found, please install Rust and Cargo to proceed."
    exit 1
fi
cargo build --release --target x86_64-unknown-linux-gnu || exit 1
cp "$git_root_dir/target/x86_64-unknown-linux-gnu/release/halloy" "$build_dir/usr/bin/"

# Now we can build the AppImage
echo "Creating AppImage..."
"$(realpath -- "$git_root_dir/assets/linux/appimagetool-x86_64.AppImage")" $build_dir "$appimage_output_dir/$appimage_name"

# Clean up build dir
echo "Cleaning up build directory..."
rm -rf "$build_dir"

if [ ! -f "$appimage_output_dir/$appimage_name" ]; then
    echo "AppImage creation failed!"
    exit 1
fi
echo "AppImage created at: $appimage_output_dir/$appimage_name"
echo "Making AppImage executable..."
chmod +x "$appimage_output_dir/$appimage_name"

echo "AppImage build process completed successfully."
exit 0





