# Building for macOS

This guide explains how to build the Halloy `.app` for macOS. If you want to install a signed release version of Halloy on macOS, then you should consult the [Installation](../installation.md) documentation instead.

## Prerequisites

- **macOS 11.0 or later**
- [**Latest Rust toolchain**](https://www.rust-lang.org/tools/install)
- **Xcode Command Line Tools** (`xcode-select --install` or <https://developer.apple.com/download/all/>)

## Steps

1. **Install Rust and Required Targets**

   Make sure you have Rust installed. Then, add the macOS targets for both Intel and Apple Silicon:

   ```sh
   rustup target add x86_64-apple-darwin
   rustup target add aarch64-apple-darwin
   ```

2. **Clone the Repository**

   If you haven’t already, clone the Halloy repository:

   ```sh
   git clone https://github.com/squidowl/halloy.git
   cd halloy
   ```

3. **Run the Build Script**

   Execute the `build-macos` script:

   ```sh
   ./scripts/build-macos.sh
   ```

   This script will:
   - Build the Halloy binary for both `x86_64` and `aarch64` architectures.
   - Combine them into a universal binary using `lipo`.
   - Copy the binary and resources into a macOS `.app` bundle template located at `assets/macos/Halloy.app`.
   - Place the `.app` bundle in `target/release/macos`.

4. **Locate the Built Application**

   After the script completes, you’ll find the generated `.app` bundle at:

   ```
   target/release/macos/Halloy.app
   ```