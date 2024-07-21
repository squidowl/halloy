#!/bin/bash
WXS_FILE="wix/main.wxs"
TARGET="x86_64-pc-windows-msvc"
VERSION=$(cat VERSION)

# install latest wix
dotnet tool install --global wix

# add required wix extension
wix extension add WixToolset.UI.wixext

# build the installer
wix build -pdbtype none -arch x64 -d PackageVersion=$VERSION $WXS_FILE -d Target=$TARGET -o halloy-installer.msi -ext WixToolset.UI.wixext