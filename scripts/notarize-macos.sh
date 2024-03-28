#!/bin/bash

RELEASE_DIR="target/release"
APP_DIR="$RELEASE_DIR/macos"
APP_NAME="Halloy.app"
APP_PATH=$APP_DIR/$APP_NAME

environment=("MACOS_NOTARIZATION_APPLE_ID" "MACOS_NOTARIZATION_TEAM_ID" "MACOS_NOTARIZATION_PWD")
for var in "${environment[@]}"; do
    if [[ -z "${!var}" ]]; then
        echo "Error: $var is not set"
        exit 1
    fi
done

echo "Create keychain profile"
xcrun notarytool store-credentials "notarytool-profile" --apple-id "$MACOS_NOTARIZATION_APPLE_ID" --team-id "$MACOS_NOTARIZATION_TEAM_ID" --password "$MACOS_NOTARIZATION_PWD"

echo "Creating temp notarization archive"
ditto -c -k --keepParent "$APP_PATH" "notarization.zip"

echo "Notarize app"
xcrun notarytool submit "notarization.zip" --keychain-profile "notarytool-profile" --wait

echo "Attach staple"
xcrun stapler staple $APP_PATH