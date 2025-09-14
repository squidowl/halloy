# Configuration

To edit configuration parameters, create a `config.toml` file located in your configuration directory:

* Windows: `%AppData%\halloy`
* Mac: `~/Library/Application Support/halloy` or `$HOME/.config/halloy`
* Linux: `$XDG_CONFIG_HOME/halloy`, `$HOME/.config/halloy` or `~/.var/app/org.squidowl.halloy/config` (Flathub)

> 💡 You can easily open the config file directory from command bar in Halloy

The specification for the configuration file format ([TOML](https://toml.io/)) can be found at [https://toml.io/](https://toml.io/).

Example config for connecting to [Libera](https://libera.chat/):

```toml
theme = "ferra"

[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]

[buffer.channel.topic]
enabled = true
```
