#! /usr/bin/env -S bash -e

# Installs to ~/.local/bin/ and ~/.local/share/ by default
# Destination directory can be overridden with the long flag:
#   --prefix=/path/to/install

  script_dir=$(dirname -- "$(realpath -- "${BASH_SOURCE[0]}")")
  git_root_dir=$(git -C "$script_dir" rev-parse --show-toplevel)

  prefix="$HOME/.local"

  for arg in "$@"; do
    case "$arg" in
    --prefix=*)
      prefix=$(awk -F'=' '{ print $2; }' <<< "$arg")
      ;;
    esac
  done

  prefix=$(realpath -- "$prefix")

# To build successfully, install packages:
#   alsa-lib-devel openssl-devel (on Fedora-based distros)
#   librust-alsa-sys-dev libssl-dev (on Debian-based distros)

  cargo install --locked --force --path "$git_root_dir" --root "$prefix"

  desktop-file-install --dir="$prefix/share/applications" "$git_root_dir/assets/linux/org.squidowl.halloy.desktop"

  cp -r "$git_root_dir/assets/linux/icons" "$prefix/share/"

  update-desktop-database "$prefix/share/applications"
  gtk-update-icon-cache -t "$prefix/share/icons/hicolor/"
