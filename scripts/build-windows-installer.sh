#!/bin/bash
WXS_FILE="wix/main.wxs"
VERSION=$(cat VERSION)

# install latest wix
dotnet tool install --global wix

# add required wix extension
wix extension add WixToolset.Ui.wixex

# build the installer
wix build -pdbtype none -arch x64 -d PackageVersion=$VERSION main.wxs -o halloy-installer.msi -ext ./.wix/extensions/WixToolset.Ui.wixext/5.0.1/wixext5/WixToolset.UI.wixext.dll
