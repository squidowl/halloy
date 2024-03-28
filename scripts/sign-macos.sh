#!/bin/bash

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Halloy.app"
APP_PATH=$APP_DIR/$APP_NAME

environment=("MACOS_CERTIFICATE" "MACOS_CERTIFICATE_PWD" "MACOS_CI_KEYCHAIN_PWD" "MACOS_CERTIFICATE_NAME" "MACOS_NOTARIZATION_APPLE_ID" "MACOS_NOTARIZATION_TEAM_ID" "MACOS_NOTARIZATION_PWD")
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
/usr/bin/codesign --force -s "$MACOS_CERTIFICATE_NAME" --options runtime $APP_PATH -v

echo "Create keychain profile"
xcrun notarytool store-credentials "notarytool-profile" --apple-id "$MACOS_NOTARIZATION_APPLE_ID" --team-id "$MACOS_NOTARIZATION_TEAM_ID" --password "$MACOS_NOTARIZATION_PWD"

echo "Creating temp notarization archive"
ditto -c -k --keepParent "$APP_PATH" "notarization.zip"

echo "Notarize app"
xcrun notarytool submit "notarization.zip" --keychain-profile "notarytool-profile" --wait

echo "Attach staple"
xcrun stapler staple $APP_PATH
