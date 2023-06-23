#!/bin/bash
set -xe

flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --noninteractive --user flathub org.freedesktop.Platform//22.08 org.freedesktop.Sdk//22.08 org.freedesktop.Sdk.Extension.rust-stable//22.08

python3 -m pip install toml aiohttp
curl -L 'https://github.com/flatpak/flatpak-builder-tools/raw/master/cargo/flatpak-cargo-generator.py' > /tmp/flatpak-cargo-generator.py
python3 /tmp/flatpak-cargo-generator.py Cargo.lock -o release/flatpak/generated-sources.json

if [ "${CI}" != "yes" ] ; then
  flatpak-builder \
    --install --force-clean --user -y \
    --state-dir /var/tmp/halloy-flatpak-builder \
    /var/tmp/halloy-flatpak-repo \
    release/flatpak/org.squidowl.halloy.json
fi
