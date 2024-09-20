# `[servers]`

Server configuration.

**Example**

```toml
[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy", "##rust"]

[servers.oftc]
nickname = "halloy-user"
server = "irc.oftc.net"
channels = ["#asahi-dev"]
```

## `nickname`

The client's nickname.

- **type**: string
- **values**: any string
- **default**: not set

## `nickname_password`

The client's NICKSERV password.

- **type**: string
- **values**: any string
- **default**: not set
 
## `nick_password_file`

Read nick_password from the file at the given path.[^1]

- **type**: string
- **values**: any string
- **default**: not set

## `nick_password_command`

Executes the command with `sh` (or equivalent) and reads `nickname_password` as the output.

- **type**: string
- **values**: any string
- **default**: not set

## `nick_identify_syntax`

The server's NICKSERV IDENTIFY syntax.

- **type**: string
- **values**: `"nick-password"`, `"password-nick"`
- **default**: not set

## `alt_nicks`

Alternative nicknames for the client, if the default is taken.  
Example: `["Foo", "Bar"]`.

- **type**: array of strings
- **values**: array of any strings
- **default**: not set

## `username`

The client's username.

- **type**: string
- **values**: any string
- **default**: not set

## `realname`

The client's real name.

- **type**: string
- **values**: any string
- **default**: not set

## `server`

The server to connect to.

- **type**: string
- **values**: any string
- **default**: not set

## `port`

The port to connect on.	

- **type**: integer
- **values** any positive integer
- **default**: `6697`

## `password`

The password to connect to the server.

- **type**: string
- **values**: any string
- **default**: not set

## `password_file`

Read password from the file at the given path.[^1]

- **type**: string
- **values**: any string
- **default**: not set

## `password_command`

Executes the command with `sh` (or equivalent) and reads `password` as the output.

- **type**: string
- **values**: any string
- **default**: not set

## `channels`

A list of channels to join on connection.
Example: `["#Foo", "#Bar"]`.

- **type**: array of strings
- **values**: array of any strings
- **default**: not set

## `channel_keys`

A mapping of channel names to keys for join-on-connect.  
Example: `channel_keys = { channel1 = "key1" }`

- **type**: map
- **values**: map with string key value
- **default**: `{}`

## `ping_time`

The amount of inactivity in seconds before the client will ping the server.

- **type**: integer
- **values**: any positive integer
- **default**: `180`

## `ping_timeout`

The amount of time in seconds for a client to reconnect due to no ping response.

- **type**: integer
- **values**: any positive integer
- **default**: `20`

## `reconnect_delay`

The amount of time in seconds before attempting to reconnect to the server when disconnected.

- **type**: integer
- **values**: any positive integer
- **default**: `10`

## `should_ghost`

Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in use.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `false`

## `ghost_sequence`

The command(s) that should be sent to NickServ to recover a nickname.

- **type**: array of strings
- **values**: array of any strings
- **default**: `["REGAIN"]`

## `umodes`

User modestring to set on connect.  
Example: `"+RB-x"`.

- **type**: string
- **values**: any string
- **default**: not set

## `use_tls`

Whether or not to use TLS. Clients will automatically panic if this is enabled without TLS support.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `dangerously_accept_invalid_certs`

When `true`, all certificate validations are skipped.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `false`

## `root_cert_path`

The path to the root TLS certificate for this server in PEM format.[^1]

- **type**: string
- **values**: any string
- **default**: not set

## `on_connect`

Commands which are executed once connected.  
Example. `["/msg NickServ IDENTIFY foo bar"]`.

- **type**: array of string
- **values**: array of any strings
- **default**: not set

## `who_poll_interval`

WHO poll interval (in seconds) for servers without away-notify.

- **type**: integer
- **values**: `5` .. `3600`
- **default**: `180`
  
## `who_retry_interval`

WHO retry interval (in seconds) for servers without away-notify.

- **type**: integer
- **values**: `5` .. `3600`
- **default**: `10`

## `monitor`

A list of nicknames to [monitor](https://ircv3.net/specs/extensions/monitor) (if IRCv3 Monitor is supported by the server).
Example: `["Foo", "Bar"]`

> 💡 Read more about [monitoring users](../../guides/monitor-users.html).

- **type**: array of strings
- **values**: array of any strings
- **default**: not set

[^1]: Shell expansions (e.g. `"~/"` → `"/home/user/"`) are not supported in path strings.
