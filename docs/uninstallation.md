# Uninstalling Halloy

## Remove binary

Remove the Halloy executable via package manager, or from the install location selected after building from source.

### macOS

For uninstalling installations from the following third party repositories in macOS

#### Homebrew

```sh
brew uninstall --cask halloy
```

#### MacPorts

```sh
sudo port uninstall halloy
```

### Linux

For uninstalling installations from the following third party repositories in Linux

#### Flatpak

```sh
flatpak uninstall org.squidowl.halloy
```

or

```sh
flatpak uninstall --delete-data org.squidowl.halloy
```

to also remove all data stored by Halloy.

#### Snapcraft

```sh
snap remove halloy
```

or

```sh
snap remove --purge halloy
```

to also remove all data stored by Halloy.

### Windows

#### Winget

```sh
winget uninstall squidowl.halloy
```

### Built from source

On modern POSIX shells, the command

```sh
command -v halloy
```

will reveal the location of the Halloy executable, which can then be removed via `rm`.

## Remove data

### History, logs, and other state data

History, logs, and other state data is stored in the following locations:

* Windows: `%AppData%\Roaming\halloy`
* macOS: `~/Library/Application Support/halloy` or `$HOME/.local/share/halloy`
* Linux: `$XDG_DATA_HOME/halloy` or `$HOME/.local/share/halloy`

### Configuration and themes

Configuration (and associated files such as sounds) and themes are stored in the following locations:

* Windows: `%AppData%\halloy`
* macOS: `~/Library/Application Support/halloy` or `$HOME/.config/halloy`
* Linux: `$XDG_CONFIG_HOME/halloy` or `$HOME/.config/halloy`

