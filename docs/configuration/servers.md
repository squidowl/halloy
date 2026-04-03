# Servers

You can define multiple server sections in the configuration file. Each server section must have a unique name, which is used as the identifier in the `[servers.<name>]` format.

Examples can be found in the following guides:

- [Example Server Configurations](../guides/example-server-configurations.md)
- [Multiple Servers](../guides/multiple-servers.md)
- [Connect with soju](../guides/connect-with-soju.md)
- [Connect with ZNC](../guides/connect-with-znc.md)

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

Read `nick_password` from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
nick_password_file = ""
```

## `nick_password_file_first_line_only`

Read `nick_password` from the first line of `nick_password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
nick_password_file_first_line_only = true
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

The server to connect to. Should not contain the protocol, port, username, or password (i.e. should look like `"irc.libera.chat"` not `"ircs://irc.libera.chat:6697"`).

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>]
server = "irc.libera.chat"
```

## `port`

The port to connect on. If you want to use a plain text port like 6667 you MUST also change the `use_tls` setting.

```toml
# Type: integer
# Values: any non-negative integer
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

## `password_file_first_line_only`

Read `password` from the first line of `password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
password_file_first_line_only = true
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

::: tip
Flatpak users need to grant host command access.

Run the following in terminal:

```sh
flatpak override org.squidowl.halloy --talk-name=org.freedesktop.Flatpak
```

Then set `password_command` to `flatpak-spawn --host <password_command>`
:::

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

A mapping of channel names to keys (passwords) for join-on-connect.

```toml
# Type: map
# Values: map with string key value
# Default: {}

[servers.<name>]
channel_keys = { "#foo" = "password" }
```

## `order_channels_by`

Ordering for channels listed in the sidebar for the current server.

- `"name"`: Sort channels by name only, ignoring chantypes (channel prefixes, e.g., `#` and `##`).
- `"name-and-prefix"`: Sort channels by name including their chantypes.
- `"config"`: Sort channels in the order they appear in your server's `channels`
  list. Any channels not in the list appear last, using default (`"name"`) sort.

If not set, the value will be taken from the sidebar config: [order_channels_by](/configuration/sidebar#order_channels_by).

```toml
# Type: string
# Values: "name", "name-and-prefix", "config"
# Default: "name"

[servers.<name>]
order_channels_by = "config"

# Example: When using "config", channels appear in this exact order:
channels = ["#rust", "#halloy", "#halloy-test"]
# Result: #rust → #halloy → #halloy-test → (any other channels are sorted by "name")
```

## `queries`

A list of queries to add to the sidebar on connection.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>]
queries = ["alice", "bob"]
```

## `ping_time`

The amount of inactivity in seconds before the client will ping the server.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 180

[servers.<name>]
ping_time = 180
```

## `ping_timeout`

The amount of time in seconds to wait for a ping response before attempting to reconnect.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 20

[servers.<name>]
ping_timeout = 20
```

## `reconnect_delay`

The amount of time in seconds before attempting to reconnect to the server when disconnected.

```toml
# Type: integer
# Values: any non-negative integer
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
# Default: true

[servers.<name>]
use_tls = true
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

Commands which are executed once connected, in the order they are specified. The `/delay <seconds>` command can be used to add a delay between commands.

```toml
# Type: array of string
# Values: array of any strings
# Default: not set

[servers.<name>]
on_connect = ["/msg NickServ IDENTIFY foo bar", "/delay 2", "/join registered-club"]
```

## `anti_flood`

The time (in milliseconds) between sending messages to servers without SAFERATE. Timing is not strictly guaranteed; small groups of messages may be allowed to be sent at a faster rate, messages may be delayed in order to be batched, automated messages are included in the queue (most at a lower priority than user messages), etc.

```toml
# Type: integer
# Values: 100 .. 60000
# Default: 2000

[servers.<name>]
anti_flood = 2000
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

WHO poll interval (in seconds) for servers without away-notify.  Specifically, the time between individual WHO requests. Will be increased automatically if the server sends a rate-limiting message.  When the server does not support SAFERATE (and [anti-flood protections](#anti_flood) are enabled) then `who_poll_interval` will be increased to more than twice [`anti_flood`](#anti_flood) if it is not already.

```toml
# Type: integer
# Values: 1 .. 3600
# Default: 2

[servers.<name>]
who_poll_interval = 2
```

## `monitor`

A list of nicknames to [monitor](https://ircv3.net/specs/extensions/monitor) (if IRCv3 Monitor is supported by the server).

::: info
Read more about [monitoring users](../guides/monitor-users.md).
:::

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

## `proxy`

Custom proxy for specified server

The logic is as follows:

- If a server proxy is provided, it will be used.
- If a server proxy is not provided, the global proxy will be used.
- If the global proxy is not provided, a plain connection will be used.

The configuration syntax and supported proxy types are similar to the global [Proxy](/configuration/proxy) but associated with the current `servers.<name>`:

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

## `autoconnect`

Whether or not to connect to the server when launching Halloy or when changing the connection details in the server configuration.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
autoconnect = true
```

## `filters`

Filter messages based on various criteria.

### `ignore`

A list of users to ignore. Users may be identified in any of these four ways:

- A string of the exact nickname to ignore in all contexts (equivalent nicknames, as defined by the server's [casemapping](https://modern.ircdocs.horse/#casemapping-parameter), will be ignored).
- A user & channel pair, written as `{ user = "nickname", channel = "#channel" }`, to ignore the user only in the specified channel.
- A regular expression, written as `{ regex = "pattern" }`, where any user whose nickname matches the regular expression will be ignored.
- A regular expression & channel pair, written as `{ regex = "pattern", channel = "#channel" }`, where any user whose nicknames matches the regular expression will be ignored in the specified channel.

```toml
# Type: array of user identifiers
# Values: array of any user identifiers
# Default: not set

[servers.<name>.filters]
ignore = [
"ignored_user", 
{ regex = '''(?i)ignored_users-.*''' },
{ user = "user_in_channel", channel = "#channel_with_user" },
{ regex = '''(?i)users_in_channel-.*''', channel = "#channel_with_users" }
]
```

### `regex`

A list of regex used to filter messages; if a match is found in the message text, then the message will be hidden.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>.filters]
regex = [
'''(?i)\bunwanted_pattern\b''',
'''^[A-Z ]+$''',
]
```

## `reroute`

Reroute selected message types within this server. See [Reroute](reroute.md) for details.

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

### `disconnect_on_failure`

Disconnect from the server if SASL authentication fails. This is useful on servers which apply a hostname cloak after identifying, such as Libera.Chat. Without this option, a failed SASL authentication would result in connecting with your real IP/hostname exposed.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>.sasl.external]
disconnect_on_failure = false
```

[^1]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
[^2]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).

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

### `password_file_first_line_only`

Read `password` from the first line of `password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>.sasl.plain]
password_file_first_line_only = true
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

### `disconnect_on_failure`

Disconnect from the server if SASL authentication fails. This is useful on servers which apply a hostname cloak after identifying, such as Libera.Chat. Without this option, a failed SASL authentication would result in connecting with your real IP/hostname exposed.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>.sasl.plain]
disconnect_on_failure = false
```

[^1]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
[^2]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).

## `confirm_message_delivery`

Whether and where to confirm delivery of sent messages, if the server supports [`echo-message`](https://ircv3.net/specs/extensions/echo-message)

### `enabled`

Control if delivery of sent messages is to be confirmed (if the server supports [`echo-message`](https://ircv3.net/specs/extensions/echo-message)).

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>.confirm_message_delivery]
enabled = true
```

### `exclude`

[Exclusion conditions](/configuration/conditions.md) in which sent message
delivery confirmation will be skipped. Inclusion conditions will take precedence
over exclusion conditions. You can also exclude all conditions by setting to
`"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: user & channel inclusion/exclusion conditions
# Default: not set

[servers.<name>.confirm_message_delivery]
exclude = "*"
```

### `include`

[Inclusion conditions](/configuration/conditions.md) in which sent message
delivery will be confirmed . Delivery of sent messages be confirmed in all
conditions (when enabled) unless explicitly excluded, so this setting is only
relevant when combined with the `exclude` setting.

```toml
# Type: inclusion/exclusion conditions
# Values: user & channel inclusion/exclusion conditions
# Default: not set

[servers.<name>.confirm_message_delivery]
include = { channels = ["#halloy"] }
```


### `typing`

Typing settings for channel and query buffers on server.

#### `share`

Control whether Halloy shares your typing status with other users on the server.

```toml
# Type: boolean
# Values: true, false
# Default: `buffer.typing.share` is used if no value is provided for the server

[servers.<name>.typing]
share = false
```

#### `show`

Control whether Halloy shows typing status from other users on the server.

```toml
# Type: boolean
# Values: true, false
# Default: `buffer.typing.show` is used if no value is provided for the server

[servers.<name>.typing]
show = true
```
