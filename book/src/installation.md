# Installing Halloy

- [Pre-built binaries](#pre-built-binaries)
- [Packaging status](#packaging-status)
- [macOS](#macos)
    - [Homebrew](#homebrew)
    - [MacPorts](#macports)
- [Linux](#linux)
    - [Flatpak](#flatpak)
    - [Snapcraft](#snapcraft)
- [Build from source](#build-from-source)

> 💡 To get the latest nightly version of Halloy, you can [build from source](#build-from-source).

## Pre-built binaries

Download pre-built binaries from [GitHub](https://github.com/squidowl/halloy/releases) page.

### Packaging status

<a href="https://repology.org/project/halloy/versions">
    <img src="https://repology.org/badge/vertical-allrepos/halloy.svg" alt="Packaging status">
</a>

### macOS

The following third party repositories are available for macOS

#### Homebrew

```
brew install --cask halloy 
```

#### MacPorts

```sh
sudo port install halloy
```

### Linux

The following third party repositories are available for Linux

#### Flatpak

[https://flathub.org/apps/org.squidowl.halloy](https://flathub.org/apps/org.squidowl.halloy)

#### Snapcraft

[https://snapcraft.io/halloy](https://snapcraft.io/halloy)

### Build from source

Clone the Halloy GitHub repository into a directory of your choice and build with cargo.

Requirements:

* [Rust toolchain](https://www.rust-lang.org/tools/install)
* [Git version control system](https://git-scm.com/)

```sh
# Clone the repository
git clone https://github.com/squidowl/halloy.git

cd halloy

# Build and run
cargo build --release
cargo run --release
```
