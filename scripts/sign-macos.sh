#!/bin/bash

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Halloy.app"

environment=("MACOS_CERTIFICATE" "MACOS_CERTIFICATE_PWD" "MACOS_CI_KEYCHAIN_PWD" "MACOS_CERTIFICATE_NAME")
for var in "${environment[@]}"; do
    if [[ -z "${!var}" ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Decoding certificate"
echo $MACOS_CERTIFICATE | base64 --decode > certificate.p12

echo "Installing cert in a new key chain"
security create-keychain -p "$MACOS_CI_KEYCHAIN_PWD" build.keychain 
security default-keychain -s build.keychain
security unlock-keychain -p "$MACOS_CI_KEYCHAIN_PWD" build.keychain
security import certificate.p12 -k build.keychain -P "$MACOS_CERTIFICATE_PWD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$MACOS_CI_KEYCHAIN_PWD" build.keychain

echo "Signing..."
/usr/bin/codesign --force -s "$MACOS_CERTIFICATE_NAME" --options runtime $APP_DIR/$APP_NAME -v