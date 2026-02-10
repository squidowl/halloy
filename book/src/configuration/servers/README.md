# Servers

You can define multiple server sections in the configuration file. Each server section must have a unique name, which is used as the identifier in the `[servers.<name>]` format.

- [Servers](#servers)
  - [Examples](#examples)
  - [Configuration](#configuration)
    - [nickname](#nickname)
    - [nick\_password](#nick_password)
    - [nick\_password\_file](#nick_password_file)
    - [nick\_password\_file\_first\_line\_only](#nick_password_file_first_line_only)
    - [nick\_password\_command](#nick_password_command)
    - [nick\_identify\_syntax](#nick_identify_syntax)
    - [alt\_nicks](#alt_nicks)
    - [username](#username)
    - [realname](#realname)
    - [server](#server)
    - [port](#port)
    - [password](#password)
    - [password\_file](#password_file)
    - [password\_file\_first\_line\_only](#password_file_first_line_only)
    - [password\_command](#password_command)
    - [channels](#channels)
    - [channel\_keys](#channel_keys)
    - [queries](#queries)
    - [ping\_time](#ping_time)
    - [ping\_timeout](#ping_timeout)
    - [reconnect\_delay](#reconnect_delay)
    - [should\_ghost](#should_ghost)
    - [ghost\_sequence](#ghost_sequence)
    - [umodes](#umodes)
    - [use\_tls](#use_tls)
    - [dangerously\_accept\_invalid\_certs](#dangerously_accept_invalid_certs)
    - [root\_cert\_path](#root_cert_path)
    - [disconnect\_on\_sasl\_failure](#disconnect_on_sasl_failure)
    - [on\_connect](#on_connect)
    - [anti\_flood](#anti_flood)
    - [who\_poll\_enabled](#who_poll_enabled)
    - [who\_poll\_interval](#who_poll_interval)
    - [monitor](#monitor)
    - [chathistory](#chathistory)
    - [proxy](#proxy)
  - [Filters](#filters)
  - [SASL Plain](#sasl-plain)
  - [SASL External](#sasl-external)
  - [Confirm Message Delivery](#confirm-message-delivery)

## Examples

Examples can be found in the following guides:
- [Example Server Configurations](../../guides/example-server-configurations.md)
- [Multiple Servers](../../guides/multiple-servers.md)
- [Connect with soju](../../guides/connect-with-soju.md)
- [Connect with ZNC](../../guides/connect-with-znc.md)

## Configuration

### nickname

The client's nickname.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nickname = ""
```

### nick_password

The client's NICKSERV password.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password = ""
```

### nick_password_file

Read `nick_password` from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password_file = ""
```

### nick_password_file_first_line_only

Read `nick_password` from the first line of `nick_password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
nick_password_file_first_line_only = true
```

### nick_password_command

Executes the command with `sh` (or equivalent) and reads `nick_password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password_command = ""
```

### nick_identify_syntax

The server's NICKSERV IDENTIFY syntax.

```toml
# Type: string
# Values: "nick-password", "password-nick"
# Default: not set

[servers.<name>]
nick_identify_syntax = ""
```

### alt_nicks

Alternative nicknames for the client, if the default is taken.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
alt_nicks = ["Foo", "Bar"]
```

### username

The client's username.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
username = ""
```

### realname

The client's real name.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
realname = ""
```

### server

The server to connect to.  Should not contain the protocol, port, username, or password (i.e. should look like `"irc.libera.chat"` not `"ircs://irc.libera.chat:6697"`).

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
server = "irc.libera.chat"
```

### port

The port to connect on. If you want to use a plain text port like 6667 you MUST also change the `use_tls` setting.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 6697

[servers.<name>]
port = 6697
```

### password

The password to connect to the server.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password = ""
```

### password_file

Read password from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password_file = ""
```

### password_file_first_line_only

Read `password` from the first line of `password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
password_file_first_line_only = true
```

### password_command

Executes the command with `sh` (or equivalent) and reads `password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
password_command = ""
```

### channels

A list of channels to join on connection.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
channels = ["#foo", "#bar"]
```

### channel_keys

A mapping of channel names to keys (passwords) for join-on-connect.

```toml
# Type: map
# Values: map with string key value
# Default: {}

[servers.<name>]
channel_keys = { "#foo" = "password" }
```

### queries

A list of queries to add to the sidebar on connection.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
queries = ["alice", "bob"]
```

### ping_time

The amount of inactivity in seconds before the client will ping the server.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 180

[servers.<name>]
ping_time = 180
```

### ping_timeout

The amount of time in seconds to wait for a ping response before attempting to reconnect.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 20

[servers.<name>]
ping_timeout = 20
```

### reconnect_delay

The amount of time in seconds before attempting to reconnect to the server when disconnected.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 10

[servers.<name>]
reconnect_delay = 10
```

### should_ghost

Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in use.

```toml
# Type: boolean
# Values: true, false
# Default: false

[servers.<name>]
should_ghost = false
```

### ghost_sequence

The command(s) that should be sent to NickServ to recover a nickname.

```toml
# Type: array of strings
# Values: array of any strings
# Default: ["REGAIN"]

[servers.<name>]
ghost_sequence = ["REGAIN"]
```

### umodes

User modestring to set on connect.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
umodes = "+RB-x"
```

### use_tls

Whether or not to use TLS. Clients will automatically panic if this is enabled without TLS support.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
use_tls = true
```

### dangerously_accept_invalid_certs

When `true`, all certificate validations are skipped.

```toml
# Type: boolean
# Values: true, false
# Default: false

[servers.<name>]
dangerously_accept_invalid_certs = false
```

### root_cert_path

The path to the root TLS certificate for this server in PEM format.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
root_cert_path = ""
```

### disconnect_on_sasl_failure

Disconnect from the server if SASL authentication fails. This is useful on servers which apply a hostname cloak after identifying, such as Libera.Chat. Without this option, a failed SASL authentication would result in connecting with your real IP/hostname exposed.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
disconnect_on_sasl_failure = false
```

### on_connect

Commands which are executed once connected, in the order they are specified. The `/delay <seconds>` command can be used to add a delay between commands.

```toml
# Type: array of string
# Values: array of any strings
# Default: not set

[servers.<name>]
on_connect = ["/msg NickServ IDENTIFY foo bar", "/delay 2", "/join registered-club"]
```

### anti_flood

The time (in milliseconds) between sending messages to servers without SAFERATE.  Timing is not strictly guaranteed;  small groups of messages may be allowed to be sent at a faster rate, messages may be delayed in order to be batched, automated messages are included in the queue (most at a lower priority than user messages), etc.

```toml
# Type: integer
# Values: 100 .. 60000
# Default: 2000

[servers.<name>]
anti_flood = 2000
```

### who_poll_enabled

Whether or not to WHO polling is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
who_poll_enabled = true
```

### who_poll_interval

WHO poll interval (in seconds) for servers without away-notify.  Specifically, the time between individual WHO requests. Will be increased automatically if the server sends a rate-limiting message.  When the server does not support SAFERATE (and anti-flood protections are enabled) then `who_poll_interval` will be increased to more than twice `anti_flood` if it is not already.

```toml
# Type: integer
# Values: 1 .. 3600
# Default: 2

[servers.<name>]
who_poll_interval = 2
```

### monitor

A list of nicknames to [monitor](https://ircv3.net/specs/extensions/monitor) (if IRCv3 Monitor is supported by the server).

> 💡 Read more about [monitoring users](../../guides/monitor-users.md).

```toml
# Type: array of string
# Values: array of any strings
# Default: not set

[servers.<name>]
monitor = ["Foo", "Bar"]
```

### chathistory

Whether or not to enable [IRCv3 Chat History](https://ircv3.net/specs/extensions/chathistory) (if it is supported by the server).

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
chathistory = true
```

### proxy

Custom proxy for specified server

The logic is as follows:

* If a server proxy is provided, it will be used.
* If a server proxy is not provided, the global proxy will be used.
* If the global proxy is not provided, a plain connection will be used.

The configuration syntax and supported proxy types are similar to the global [Proxy](../proxy/) but associated with the current `servers.<name>`:

```toml
[servers.<name>.proxy.http]
host = "192.168.1.100"
port = 1080
username = "username"
password = "password"
```

or

```toml
[servers.<name>.proxy.socks5]
host = "192.168.1.100"
port = 1080
username = "username"
password = "password"
```

## [Filters](filters.md)

Filter messages based on various criteria

## [SASL Plain](sasl-plain.md)

Plain SASL auth using a username and password

## [SASL External](sasl-external.md)

External SASL auth uses a PEM encoded X509 certificate.

## [Confirm Message Delivery](confirm-message-delivery.md)

Whether and where to confirm delivery of sent messages, if the server supports [`echo-message`](https://ircv3.net/specs/extensions/echo-message)

[^1]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
[^2]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).
