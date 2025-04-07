#!/bin/bash
WXS_FILE="wix/main.wxs"
HALLOY_VERSION=$(cat VERSION).0

# build the binary
scripts/build-windows.sh

# install latest wix
dotnet tool install --global wix --version 5.0.2

# add required wix extension
wix extension add WixToolset.UI.wixext/5.0.2

# build the installer
wix build -pdbtype none -arch x64 -d PackageVersion=$HALLOY_VERSION $WXS_FILE -o target/release/halloy-installer.msi -ext WixToolset.UI.wixext