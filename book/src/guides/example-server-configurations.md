# Example Server Configurations

## [Libera](https://libera.chat/)

```toml
[servers.Libera]
server = "irc.libera.chat"
use_tls = true # default value
port = 6697 # default value when use_tls = true

nickname = "halloy-user"

channels = ["#halloy"]
```

## [OFTC](https://oftc.net/)

```toml
[servers.OFTC]
server = "irc.oftc.net"
use_tls = true # default value
port = 6697 # default value when use_tls = true

nickname = "halloy-user"
```

## [SlashNET](https://www.slashnet.org/)

```toml
[servers.SlashNET]
server = "irc.slashnet.org"
use_tls = false
port = 6667 # default value when use_tls = false

nickname = "halloy-user"
```

## [Undernet](https://www.undernet.org/)

```toml
[servers.Undernet]
server = "irc.undernet.org"
use_tls = false
port = 6667 # default value when use_tls = false

nickname = "halloy-user"
```
