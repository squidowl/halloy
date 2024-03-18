# Configuration

To edit configuration parameters, create a `config.toml` file located in your configuration directory:

* Mac: `~/Library/Application Support/halloy`
* Windows: `%AppData%\halloy\config.toml`
* Linux: `$XDG_CONFIG_HOME` or `$HOME/.config`

> 💡 You can easily open the config file directory from command bar in Halloy

Example config for connecting to [libera.chat](https://libera.chat/):

```toml
theme = "ferra"

[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]

[buffer.channel.topic]
enabled = true
```

You can also run Halloy in portable mode, if there is a `config.toml` configuration file in the same directory as the running executable.