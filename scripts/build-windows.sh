# Deprecated for now.
# We should later use it for portable version of Halloy.

#!/bin/bash
EXE_NAME="halloy.exe"
TARGET="x86_64-pc-windows-msvc"
HALLOY_VERSION=$(grep -q '\..*\.' VERSION && cat VERSION || echo "$(cat VERSION).0")
PROFILE="packaging"

# update package version on Cargo.toml
if ! command -v cargo-set-version &> /dev/null; then
    cargo install cargo-edit
fi
cargo set-version $HALLOY_VERSION

# build binary
rustup target add $TARGET
cargo build --profile $PROFILE --locked --target=$TARGET
cp -fp target/$TARGET/$PROFILE/$EXE_NAME target/$PROFILE
