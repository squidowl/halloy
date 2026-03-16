# Multiple Servers

Creating multiple `[servers]` sections lets you connect to multiple servers.  
All configuration options can be found in the [servers configuration section](../configuration/servers/).

```toml
[servers.Libera]
server = "irc.libera.chat"
use_tls = true # default value
port = 6697 # default value when use_tls = true

nickname = "nickname-on-libera"

channels = ["#halloy"]

[servers.OFTC]
server = "irc.oftc.net"
use_tls = true # default value
port = 6697 # default value when use_tls = true

nickname = "nickname-on-oftc"
```
