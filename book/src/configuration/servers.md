# Servers

## `[servers]` Section

Example

```toml
[servers.liberachat]
nickname = "halloy-user"
server = "irc.libera.chat"
channels = ["#halloy"]
```

| Key                                | Description                                                                                         | Default     |
| :--------------------------------- | :-------------------------------------------------------------------------------------------------- | :---------- |
| `nickname`                         | The client's nickname.                                                                              | `""`        |
| `nick_password`                    | The client's NICKSERV password.                                                                     | `""`        |
| `nick_password_file`               | Alternatively read `nick_password` from the file at the given path.                                 | `""`        |
| `nick_identify_syntax`             | The server's NICKSERV IDENTIFY syntax. Can be `"nick-then-password"` or `"password-then-nick"`.     | `""`        |
| `alt_nicks`                        | Alternative nicknames for the client, if the default is taken.                                      | `[""]`      |
| `username`                         | The client's username.                                                                              | `""`        |
| `realname`                         | The client's real name.                                                                             | `""`        |
| `server`                           | The server to connect to.                                                                           | `""`        |
| `port`                             | The port to connect on.                                                                             | `6697`      |
| `password`                         | The password to connect to the server.                                                              | `""`        |
| `password_file`                    | Alternatively read `password` from the file at the given path.                                      | `""`        |
| `channels`                         | A list of channels to join on connection.                                                           | `[""]`      |
| `channel_keys`                     | A mapping of channel names to keys for join-on-connect.                                             | `{}`        |
| `ping_time`                        | The amount of inactivity in seconds before the client will ping the server.                         | `180`       |
| `ping_timeout`                     | The amount of time in seconds for a client to reconnect due to no ping response.                    | `20`        |
| `reconnect_delay`                  | The amount of time in seconds before attempting to reconnect to the server when disconnected.       | `10`        |
| `should_ghost`                     | Whether the client should use NickServ GHOST to reclaim its primary nickname if it is in use.       | `false`     |
| `ghost_sequence`                   | The command(s) that should be sent to NickServ to recover a nickname.                               | `["GHOST"]` |
| `umodes`                           | User modestring to set on connect. Example: `"+RB-x"`.                                              | `""`        |
| `use_tls`                          | Whether or not to use TLS. Clients will automatically panic if this is enabled without TLS support. | `true`      |
| `dangerously_accept_invalid_certs` | On `true`, all certificate validations are skipped. Defaults to `false`.                            | `false`     |
| `root_cert_path`                   | The path to the root TLS certificate for this server in PEM format.                                 | `""`        |
| `on_connect`                       | Commands which are executed once connected. Example. `["/msg NickServ IDENTIFY foo bar"]`.          | `[]`        |
| `who_poll_interval`                | WHO poll interval (in seconds) for servers without away-notify.                                     | `180`[^1]   |
| `who_retry_interval`               | WHO retry interval (in seconds) for servers without away-notify.                                    | `10`[^1]    |

[^1]: Limited between `5` and `3600` seconds.

## `[servers.sasl]` Section

### `[sasl.plain]`:

```toml
[servers.liberachat.sasl.plain]
username = "<string>"
password = "<string>"
```

| Key             | Description                                                       | Default |
| :---------------| :---------------------------------------------------------------- | :------ |
| `username`      | The account name used for authentication.                         | `""`    |
| `password`      | The password associated with the account used for authentication. | `""`    |
| `password_file` | Alternatively read `password` from the file at the given path.    | `""`    |


### `[sasl.external]`

```toml
[servers.liberachat.sasl.external]
cert = "<string>"
key = "<string>"
```

> 💡 External SASL auth uses a PEM encoded X509 certificate. [Reference](https://libera.chat/guides/certfp).

| Key    | Description                                                             | Value |
| :----- | :---------------------------------------------------------------------- | :---- |
| `cert` | The path to PEM encoded X509 user certificate for external auth         | `""`  |
| `key`  | The path to PEM encoded PKCS#8 private key for external auth (optional) | `""`  |
