# Building for Flatpak

This guide will help you to build and/or test pre-released commits of Halloy
for flatpak.

If you haven't done so already, clone the [Halloy repository][halloy-repo] to
your local machine.

## Requirements

- [flatpak-builder][flatpak-builder]
- [flatpak-cargo-generator][flatpak-cargo-generator]

Be sure to install all the dependencies for the above tools.

## Generating Flatpak Build Sources File

To generate the `generated-sources.json` file that is used by the flatpak
manifest, run the following command:

```bash
python3 <flatpak-builder-tools-path>/cargo/flatpak-cargo-generator.py -o <halloy-checkout-path>/assets/flatpak/generated-sources.json
```

## Building and Installing Flatpak Locally

Finally, to build and install the flatpak locally, run the following command:

```bash
flatpak-builder --force-clean --user --install-deps-from=flathub --repo=.flatpak-builder/repo --install .flatpak-builder/build <halloy-checkout-path>/assets/flatpak/org.squidowl.halloy.json
```

Your flatpak should now be built, installed locally and ready for use.

Happy testing!

## Releasing to Flathub

Halloy's flatpaks are released via [Flathub][halloy-flathub-repo].

Start by cloning the flathub repo for Halloy.
Every release has two requirements:

1. The `generated-sources.json` file must be up to date. You can generate it
   from the latest release tag via the commands above.
2. The flatpak manifest file (`org.squidowl.halloy.json`) must be updated to
   point to the latest release tag of Halloy.

The caveat for #2: the version you're wanting to release must be tagged first.
We use the sha256sum of the tagged tarball in the build manifest.

For example, for release `2025.6`, we would need to:

```bash
# Download the tagged tarball
wget https://github.com/squidowl/halloy/archive/refs/tags/2025.6.tar.gz

# Get the sha256sum of the tarball
sha256sum 2025.6.tar.gz | awk '{print $1}'
```

The `url` and the `sha256` fields for `modules.0.sources.0` in the manifest file
should be updated along with an updated `generated-sources.json` file. After
that, you can create a pull request to the [Flathub repository][halloy-flathub-repo]
with the updated files.

See the pull request for the [2025.6 release][halloy-flathub-2025.6-pr] for an
example.

[halloy-repo]: https://github.com/squidowl/halloy
[flatpak-builder]: https://docs.flatpak.org/en/latest/flatpak-builder.html
[flatpak-cargo-generator]: https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo
[halloy-flathub-repo]: https://github.com/flathub/org.squidowl.halloy/
[halloy-flathub-2025.6-pr]: https://github.com/flathub/org.squidowl.halloy/pull/26
