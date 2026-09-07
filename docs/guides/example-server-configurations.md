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
sasl.plain.password_file = "/path/to/super-secret-password-file" # file containing password used for authentication
```

## [IRCCloud](https://www.irccloud.com/)

```toml
[servers.Something] # use the name of the network you're connecting to through IRCCloud, like OFTC or Libera
server = "bnc.irccloud.com"
use_tls = true
port = 6697

nickname = "registered-user" # your name on the server
realname = "registered-user" # must be set, may match your nickname or your real name or something else
password = "bnc@clientid:random-string" # password from "Connect with another IRC client" dialogue

channels = ["#halloy"] # channel(s) joined on launch
```

## [OFTC](https://oftc.net/)

### Unregistered

```toml
[servers.OFTC] # server name OFTC used in sidebar/UI
server = "irc.oftc.net" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "halloy-user" # your name on the server
```

### [Registered with SSL CertFP](https://www.oftc.net/NickServ/CertFP/#automatically-identifying-using-ssl--certfp)

```toml
[servers.OFTC] # server name OFTC used in sidebar/UI
server = "irc.oftc.net" # address of server
use_tls = true # TLS is recommended when available
port = 6697 # default port when use_tls = true

nickname = "registered-user" # your name on the server

sasl.external.cert = "/path/to/nick.cer" # path to your certificate
sasl.external.key = "/path/to/nick.key" # path to your private key
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
