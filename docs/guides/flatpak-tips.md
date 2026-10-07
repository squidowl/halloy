# Flatpak Tips

Here are a collection of tips for using the Flatpak version of Halloy.

## Wayland clipboard issues

There are currently [Wayland clipboard issues](https://github.com/1Password/arboard/issues/223)
with some desktop environments.

There is a workaround that might work, but first test this:

```sh
flatpak run --nosocket=wayland org.squidowl.halloy
```

If this works, you can set the override permanently:
```sh
flatpak override --user --nosocket=wayland org.squidowl.halloy
```

Your mileage may vary, but turning off Wayland like this may cause UI issues.

You can follow our [issue here](https://github.com/flathub/org.squidowl.halloy/issues/52).

## Keyring access

To access keyrings while running in Flatpak, you will need to manually grant your install of
Halloy access to the system keyring. You can do this by running:


```bash
flatpak override --user --talk-name=org.freedesktop.secrets org.squidowl.halloy
```

## Configuration in `$HOME` paths

When using Flatpak, the location of your Halloy data is `~/.var/app/org.squidowl.halloy`.

If you want to access your configuration data via the `$HOME` paths, this
section will help you do so.

If you've not been using Flatpak to configure Halloy yet, or you want to use
existing configuration located in your `$HOME` paths, then skip to the
[override section](./flatpak-tips#sharing-data-with-home-paths).

### Existing Flatpak installation and usage

If you've been running and using Flatpak, then your data folders
(`~/.var/app/org.squidowl.halloy`) needs to be symlinked to your `$HOME`
folders.

```sh
# rename your existing Flatpak config & data
mv ~/.var/app/org.squidowl.halloy/cache/halloy ~/.var/app/org.squidowl.halloy/cache/halloy.bak
mv ~/.var/app/org.squidowl.halloy/config/halloy ~/.var/app/org.squidowl.halloy/config/halloy.bak
mv ~/.var/app/org.squidowl.halloy/data/halloy ~/.var/app/org.squidowl.halloy/data/halloy.bak

# renaming any existing data to prevent being overwritten
mv ~/.cache/halloy ~/.cache/halloy.bak
mv ~/.config/halloy ~/.config/halloy.bak
mv ~/.local/share/halloy ~/.local/share/halloy.bak

# restore your data from the Flatpak to the $HOME paths
cp -R ~/.var/app/org.squidowl.halloy/cache/halloy.bak ~/.cache/halloy
cp -R ~/.var/app/org.squidowl.halloy/config/halloy.bak ~/.config/halloy
cp -R ~/.var/app/org.squidowl.halloy/data/halloy.bak ~/.local/share/halloy
```

### Sharing data with `$HOME` paths

Flatpak by default does not allow access to the `$HOME` directory. You will need
to grant access and then symlink the Halloy paths to the correct locations.

```sh
flatpak override --user \
  --filesystem=~/.cache/halloy:create \
  --filesystem=~/.config/halloy:create \
  --filesystem=~/.local/share/halloy:create \
  org.squidowl.halloy

ln -s ~/.cache/halloy ~/.var/app/org.squidowl.halloy/cache/halloy
ln -s ~/.config/halloy ~/.var/app/org.squidowl.halloy/config/halloy
ln -s ~/.local/share/halloy ~/.var/app/org.squidowl.halloy/data/halloy
```

Your configuration data will now be shared with the `$HOME` paths.
