# Flatpak Tips

Here are a collection of tips for using the Flatpak version of Halloy.

## Configuration in `$HOME` paths

When using Flatpak, the default location of your Halloy data is `~/.var/app/org.squidowl.halloy`.

If you would rather use the `$HOME` paths instead, the following should help you
do so.

If you've not been using Flatpak to configure Halloy yet, or you want to use
existing configuration located in your `$HOME` paths, then skip to the
[override section](./flatpak-tips#accessing-home-paths-in-flatpak).

### Existing Flatpak installation and usage

If you've been configuring Halloy via the Flatpak data folders
(`~/.var/app/org.squidowl.halloy`), and you want to switch to the `$HOME`
folders, start by backing up your configuration:

```sh
# backup your existing Halloy configuration & data
mv ~/.var/app/org.squidowl.halloy/cache/halloy ~/.var/app/org.squidowl.halloy/cache/halloy.bak
mv ~/.var/app/org.squidowl.halloy/config/halloy ~/.var/app/org.squidowl.halloy/config/halloy.bak
mv ~/.var/app/org.squidowl.halloy/data/halloy ~/.var/app/org.squidowl.halloy/data/halloy.bak

# backing these up in case they've any data in them
mv ~/.cache/halloy ~/.cache/halloy.bak
mv ~/.config/halloy ~/.config/halloy.bak
mv ~/.local/share/halloy ~/.local/share/halloy.bak

# restore your configuration from the flatpaks to the $HOME paths
cp -R ~/.var/app/org.squidowl.halloy/cache/halloy.bak ~/.cache/halloy
cp -R ~/.var/app/org.squidowl.halloy/config/halloy.bak ~/.config/halloy
cp -R ~/.var/app/org.squidowl.halloy/data/halloy.bak ~/.local/share/halloy
```

### Accessing `$HOME` paths in Flatpak

Flatpak by default does not allow access to the `$HOME` directory. You will need
to grant access and then symlink the Halloy paths to the correct locations.

```sh
flatpak override --user \
  --filesystem=~/.cache/halloy:rw \
  --filesystem=~/.config/halloy:rw \
  --filesystem=~/.local/share/halloy:rw \
  org.squidowl.halloy

ln -s ~/.cache/halloy ~/.var/app/org.squidowl.halloy/cache/halloy
ln -s ~/.config/halloy ~/.var/app/org.squidowl.halloy/config/halloy
ln -s ~/.local/share/halloy ~/.var/app/org.squidowl.halloy/data/halloy
```

You can now launch the Flatpak and your configuration will be loaded from the
`$HOME` folders.

## Keyring access

To access keyrings while running in Flatpak, you will need to manually grant your install of
Halloy access to the system keyring. You can do this by running:


```bash
flatpak override --user --talk-name=org.freedesktop.secrets org.squidowl.halloy
```

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
