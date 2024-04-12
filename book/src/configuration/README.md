# Configuration

To edit configuration parameters, create a `config.toml` file located in your configuration directory:

* Windows: `%AppData%\halloy`
* Mac: `~/Library/Application Support/halloy` or `$HOME/.config/halloy`
* Linux: `$XDG_CONFIG_HOME` or `$HOME/.config`

> 💡 You can easily open the config file directory from command bar in Halloy

Example config for connecting to [Libera](https://libera.chat/):

```toml
[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]

[buffer.channel.topic]
enabled = true
```
