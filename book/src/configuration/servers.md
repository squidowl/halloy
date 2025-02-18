# `[servers]`

You can define multiple server sections in the configuration file. Each server section must have a unique name, which is used as the identifier in the `[servers.<name>]` format.

Eg: 

```toml
[servers.quakenet]
# ...
```

> 💡 For a multiple server example see [here](../guides/multiple-servers.html)

## `nickname`

The client's nickname.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nickname = ""
```

## `nick_password`

The client's NICKSERV password.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password = ""
```

## `nick_password_file`

Read nick_password from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password_file = ""
```

## `nick_password_command`

Executes the command with `sh` (or equivalent) and reads `nick_password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password_command = ""
```

## `nick_identify_syntax`

The server's NICKSERV IDENTIFY syntax.

```toml
# Type: string
# Values: "nick-password", "password-nick"
# Default: not set

[servers.<name>]
nick_identify_syntax = ""
```

## `alt_nicks`

Alternative nicknames for the client, if the default is taken.  

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
alt_nicks = ["Foo", "Bar"]
```

## `username`

The client's username.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
username = ""
```


## `realname`

The client's real name.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
realname = ""
```

## `server`

The server to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
server = "irc.libera.chat"
```

## `port`

The port to connect on.	

```toml
# Type: integer
# Values: any positive integer
# Default: 6697

[servers.<name>]
port = 6697
```

## `password`

The password to connect to the server.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password = ""
```

## `password_file`

Read password from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password_file = ""
```

## `password_command`

Executes the command with `sh` (or equivalent) and reads `password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password_command = ""
```

## `channels`

A list of channels to join on connection.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
channels = ["#foo", "#bar"]
```

## `channel_keys`

A mapping of channel names to keys for join-on-connect.

```toml
# Type: map
# Values: map with string key value
# Default: {}

[servers.<name>]
channel_keys = { channel1 = "key1" }
```

## `ping_time`

The amount of inactivity in seconds before the client will ping the server.

```toml
# Type: integer
# Values: any positive integer
# Default: 180

[servers.<name>]
ping_time = 180
```

## `ping_timeout`

The amount of time in seconds for a client to reconnect due to no ping response.

```toml
# Type: integer
# Values: any positive integer
# Default: 20

[servers.<name>]
ping_timeout = 20
```

## `reconnect_delay`

The amount of time in seconds before attempting to reconnect to the server when disconnected.

```toml
# Type: integer
# Values: any positive integer
# Default: 10

[servers.<name>]
reconnect_delay = 10
```

## `should_ghost`

Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in use.

```toml
# Type: boolean
# Values: true, false
# Default: false

[servers.<name>]
should_ghost = false
```

## `ghost_sequence`

The command(s) that should be sent to NickServ to recover a nickname.

```toml
# Type: array of strings
# Values: array of any strings
# Default: ["REGAIN"]

[servers.<name>]
ghost_sequence = ["REGAIN"]
```

## `umodes`

User modestring to set on connect.  

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
umodes = "+RB-x"
```

## `use_tls`

Whether or not to use TLS. Clients will automatically panic if this is enabled without TLS support.

```toml
# Type: boolean
# Values: true, false
# Default: false

[servers.<name>]
use_tls = false
```

## `dangerously_accept_invalid_certs`

When `true`, all certificate validations are skipped.

```toml
# Type: boolean
# Values: true, false
# Default: false

[servers.<name>]
dangerously_accept_invalid_certs = false
```

## `root_cert_path`

The path to the root TLS certificate for this server in PEM format.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
root_cert_path = ""
```

## `on_connect`

Commands which are executed once connected. 

```toml
# Type: array of string
# Values: array of any strings
# Default: not set

[servers.<name>]
on_connect = ["/msg NickServ IDENTIFY foo bar"]
```
  
## `who_poll_enabled`

Whether or not to WHO polling is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
who_poll_enabled = true
```

## `who_poll_interval`

WHO poll interval (in seconds) for servers without away-notify.  Specifically, the time between individual WHO requests. Will be increased automatically if the server sends a rate-limiting message.

```toml
# Type: integer
# Values: 1 .. 3600
# Default: 2

[servers.<name>]
who_poll_interval = 2
```


## `monitor`

A list of nicknames to [monitor](https://ircv3.net/specs/extensions/monitor) (if IRCv3 Monitor is supported by the server).

> 💡 Read more about [monitoring users](../guides/monitor-users.html).

```toml
# Type: array of string
# Values: array of any strings
# Default: not set

[servers.<name>]
monitor = ["Foo", "Bar"]
```

## `chathistory`

Whether or not to enable [IRCv3 Chat History](https://ircv3.net/specs/extensions/chathistory) (if it is supported by the server).

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
chathistory = true
```

## `sasl.plain`

Plain SASL auth using a username and password 

### `username`

The account name used for authentication.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
username = "username"
```

### `password`

The password associated with the account used for authentication.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password = "password"
```

### `password_file`

Read `password` from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password_file = ""
```

### `password_command`

Executes the command with `sh` (or equivalent) and reads `password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password_command = ""
```

## `sasl.external`

External SASL auth uses a PEM encoded X509 certificate. [Reference](https://libera.chat/guides/certfp).

### `cert`

The path to PEM encoded X509 user certificate for external auth.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.external]
cert = "/path/to/your/certificate.pem"
```

### `key`

The path to PEM encoded PKCS#8 private key for external auth (optional).[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.external]
key = "/path/to/your/private_key.pem"
```

[^1]: Shell expansions (e.g. `"~/"` → `"/home/user/"`) are not supported in path strings.
[^2]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
