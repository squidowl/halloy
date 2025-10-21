# Configuration

Halloy uses a TOML file for configuration called `config.toml`.  
A default file is created when you launch Halloy for the first time. The location depends on your system:

* Windows: `%AppData%\halloy`
* Mac: `~/Library/Application Support/halloy` or `$HOME/.config/halloy`
* Linux: `$XDG_CONFIG_HOME/halloy`, `$HOME/.config/halloy` or `$HOME/.var/app/org.squidowl.halloy/config` (Flatpak)

> 💡 Most configuration changes can be applied by reloading the configuration file from the sidebar menu, [keyboard shortcut](./configuration/keyboard.md), or the command bar

The specification for the configuration file format ([TOML](https://toml.io/)) can be found at [https://toml.io/](https://toml.io/).

Example configuration for connecting to [Libera](https://libera.chat/):

```toml
theme = "ferra"

[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]

[buffer.channel.topic]
enabled = true
```
