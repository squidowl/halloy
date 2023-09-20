#!/bin/bash
WXS_FILE="wix/main.wxs"
VERSION=$(cat VERSION)

# update version and build
sed -i '' -e "s/{{ VERSION }}/$VERSION/g" "$WXS_FILE"

# install wix tools, and ensure paths are set
choco install wixtoolset -y --force --version=3.11.2
$env:Path += ';C:\Program Files (x86)\Wix Toolset v3.11\bin'

# build msi installer
cargo install cargo-wix
cargo wix --nocapture --package halloy -o target/release/halloy-installer.msi
