#!/usr/bin/env bash

version="$1"
release_date="$(date +%F)"

changelog_file="CHANGELOG.md"
version_file="VERSION"
appdata_file="assets/linux/org.squidowl.halloy.appdata.xml"

VERSION="$version" DATE="$release_date" perl -0pi -e '
  s/^# Unreleased\n/"# Unreleased\n\n# $ENV{VERSION} ($ENV{DATE})\n"/me
' "$changelog_file"

printf '%s\n' "$version" > "$version_file"

VERSION="$version" DATE="$release_date" perl -0pi -e '
  s{<release version="[^"]+"\n\s+date="[^"]+"\s*/>}{qq{<release version="$ENV{VERSION}"\n                 date="$ENV{DATE}" />}}e
' "$appdata_file"

echo "Release set to $1"
