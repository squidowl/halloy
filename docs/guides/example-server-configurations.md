# Example Server Configurations

## [Libera](https://libera.chat/)

### Unregistered

```toml
[servers.Libera] # server name Libera used in sidebar
server = "irc.libera.chat" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "halloy-user" # your name on the server

channels = ["#halloy"] # channel(s) joined on launch
```

### [Registered](https://libera.chat/guides/registration)

```toml
[servers.Libera] # server name Libera used in sidebar/UI
server = "irc.libera.chat" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "registered-user" # your name on the server

channels = ["#registered-users-clubhouse"] # channel(s) joined on launch

sasl.plain.username = "registered-user" # name used for authentication; often the same as nickname
sasl.plain.password_file = "super-secret-password" # password used for authentication
```

## [OFTC](https://oftc.net/)

```toml
[servers.OFTC] # server name OFTC used in sidebar/UI
server = "irc.oftc.net" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "halloy-user" # your name on the server
```

## [SlashNET](https://www.slashnet.org/)

```toml
[servers.SlashNET] # server name SlashNET used in sidebar/UI
server = "irc.slashnet.org" # address of server
use_tls = false # disabled because TLS is not available
port = 6667 # default port when use_tls = false

nickname = "halloy-user" # your name on the server
```

## [Undernet](https://www.undernet.org/)

```toml
[servers.Undernet] # server name Undernet used in sidebar/UI
server = "irc.undernet.org" # address of server
use_tls = false # disabled because TLS is not available
port = 6667 # default port when use_tls = false

nickname = "halloy-user" # your name on the server
```

## [2600net](https://scuttled.net/)

```toml
[servers.2600net] # server name 2600net used in sidebar/UI
server = "irc.scuttled.net" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "halloy-user" # your name on the server
```
