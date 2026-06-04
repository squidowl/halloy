#!/bin/bash
set -euo pipefail

ISS_FILE="inno/halloy.iss"
HALLOY_VERSION=$(grep -q '\..*\.' VERSION && cat VERSION || echo "$(cat VERSION).0")
PROFILE="packaging"
NIGHTLY=0

find_iscc() {
    if [ -n "${ISCC:-}" ]; then
        echo "$ISCC"
    elif command -v iscc &> /dev/null; then
        command -v iscc
    fi
}

for arg in "$@"; do
    case "$arg" in
        --nightly)
            NIGHTLY=1
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            exit 1
            ;;
    esac
done

# build the binary
scripts/build-windows.sh

ISCC_BIN="$(find_iscc)"

if [ -z "$ISCC_BIN" ]; then
    echo "Could not find ISCC.exe. Install Inno Setup 6 or set ISCC to the compiler path." >&2
    exit 1
fi

export HALLOY_VERSION
export HALLOY_SOURCE_DIR="$PWD"
export HALLOY_OUTPUT_DIR="$PWD/target/$PROFILE"

if [ "$NIGHTLY" = "1" ]; then
    DEFAULT_OUTPUT_BASE_FILENAME="halloy-nightly-installer"
else
    DEFAULT_OUTPUT_BASE_FILENAME="halloy-installer"
fi
export HALLOY_OUTPUT_BASE_FILENAME="${HALLOY_OUTPUT_BASE_FILENAME:-$DEFAULT_OUTPUT_BASE_FILENAME}"

# build the installer
"$ISCC_BIN" "$ISS_FILE"
