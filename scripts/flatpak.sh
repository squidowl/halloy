#!/bin/bash
set -xe

flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --noninteractive --user flathub org.freedesktop.Platform//25.08 org.freedesktop.Sdk//25.08 org.freedesktop.Sdk.Extension.rust-stable//25.08

# Check and install only missing packages
missing=()
for pkg in toml aiohttp; do
  python3 -c "import $pkg" >/dev/null 2>&1 || missing+=("$pkg")
done

if ((${#missing[@]})); then
  python3 -m pip install "${missing[@]}"
fi

if [ ! -f /tmp/flatpak-cargo-generator.py ] ; then
  curl -L 'https://github.com/flatpak/flatpak-builder-tools/raw/master/cargo/flatpak-cargo-generator.py' > /tmp/flatpak-cargo-generator.py
fi
python3 /tmp/flatpak-cargo-generator.py Cargo.lock -o assets/flatpak/generated-sources.json

if [ "${CI}" != "yes" ] ; then
  flatpak-builder \
    --install --force-clean --user \
    --install-deps-from=flathub \
    --repo=/var/tmp/halloy-flatpak-repo \
    --state-dir=/var/tmp/halloy-flatpak-state \
    /var/tmp/halloy-flatpak-build assets/flatpak/org.squidowl.halloy.json
fi
