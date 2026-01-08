# Configuration

Halloy uses a TOML file for configuration called `config.toml`.  
A default file is created when you launch Halloy for the first time. The location depends on your system:

* Windows: `%AppData%\halloy`
* Mac: `~/Library/Application Support/halloy` or `$HOME/.config/halloy`
* Linux: `$XDG_CONFIG_HOME/halloy`, `$HOME/.config/halloy` or `$HOME/.var/app/org.squidowl.halloy/config` (Flatpak)

> 💡 Most configuration changes can be applied by reloading the configuration file from the sidebar menu, [keyboard shortcut](./configuration/keyboard.md), or the command bar

The specification for the configuration file format ([TOML](https://toml.io/)) can be found at [https://toml.io/](https://toml.io/).

See the following guides for example configurations:
- [Example Server Configurations](./guides/example-server-configurations.md)
- [Multiple Servers](./guides/multiple-servers.md)
- [Connect with soju](./guides/connect-with-soju.md)
- [Connect with ZNC](./guides/connect-with-znc.md)
