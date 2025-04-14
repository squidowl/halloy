# Multiple servers

Creating multiple `[servers]` sections lets you connect to multiple servers.  
All configuration options can be found [here](../configuration/servers.md).

```toml
[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]

[servers.oftc]
nickname = "halloy-user"
server = "irc.oftc.net"
channels = ["#asahi-dev"]
```
