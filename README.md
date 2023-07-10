<div align="center">
  
# Halloy
![halloy boje](https://github.com/squidowl/halloy/assets/2248455/414d4466-b9ca-446b-901c-68acfcdff5e8)

</div>

![halloy](./assets/animation.gif)

Halloy is an open-source IRC client written in Rust, with the Iced GUI library. It aims to provide a simple and fast client for Mac, Windows, and Linux platforms.

<a href="https://github.com/iced-rs/iced">
  <img src="https://gist.githubusercontent.com/hecrj/ad7ecd38f6e47ff3688a38c79fd108f0/raw/74384875ecbad02ae2a926425e9bcafd0695bade/color.svg" width="130px">
</a>

## Download

Prebuilt binaries for macOS and Windows can be downloaded from [GitHub Releases](https://github.com/squidowl/halloy/releases). For Linux, please use [Flatpak]( https://flathub.org/apps/org.squidowl.halloy).


## Build

To build Halloy from source

1. Clone the repository:

```
git clone https://github.com/squidowl/halloy.git
```

2. Build the project:

```
cd halloy
cargo build --release
```

3. Run Halloy:

```
cargo run --release
```

## Capabilities

Halloy supports the following IRCv3.2 capabilities

- `batch`
- `server-time`
- `labeled-response`
- `echo-message` (server must also support `labeled-response`)

## Why?
<div align="center">
  <a href="https://xkcd.com/1782/">
    <img src="https://imgs.xkcd.com/comics/team_chat.png" title="2078: He announces that he's finally making the jump from screen+irssi to tmux+weechat.">
  </a>
</div>


## License

Halloy is released under the GPL-3.0 License. For more details, see the [LICENSE](LICENSE) file.

## Disclaimer

Halloy is still in the early stages of development. Bugs and incomplete features may be present. Use it at your own risk.

## Contact

For any questions, suggestions, or issues, please open an issue on the [GitHub repository](https://github.com/squidowl/halloy/issues).
