# Inclusion/Exclusion Conditions

Inclusion/exclusion conditions can be specified by arrays of users, channels,
servers, [server message types](/configuration/buffer/server-messages/#types),
and/or combined criteria. A combined criterion can each include a user, channel,
server, and/or [server message
type](/configuration/buffer/server-messages/#types), but will only be matched if
**all** are matched. The conditions are met if **any** individual condition is
matched.

Inclusion conditions will take precedence over exclusion conditions.

You can specify all conditions by setting to `"all"` or `"*"`.

## Types

| **Condition Type** | **Description**                                                                                                               |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------- |
| `users`            | An array of nicknames.  Conditions will apply to messages sent by or queries with a user with one of the specified nicknames. |
| `channels`         | An array of channel names.  Conditions will apply to messages in any of the specified channels.                               |
| `servers`          | An array of server names.  Conditions will apply to all messages on any of the specified servers.                             |
| `server_messages`  | An array of [server message types](/configuration/buffer/server-messages/#types). Conditions will apply to server messages of any of the specified types. |
| `criteria`         | Each criterion in the array can have a specified `user`, `channel`, `server`, and/or `server_message`. All specified fields must match for the criterion to match. |

## Examples

This example excludes all messages in `#halloy` except for messages from
`GH-Bot`.

```toml
include = { users = ["#GH-Bot"] }
exclude = { channels = ["#halloy"] }
```

This example excludes messages from `GH-Bot` and `ChanServ` in `#halloy`.

```toml
exclude = { criteria = [{ channel = "#halloy", user = "GH-Bot" }, { channel = "#halloy", user = "ChanServ" }] }
```

Using a regular TOML table can help with legibility, as in this example
excluding notifications for highlights by `GH-Bot` and `ChanServ` in `#halloy`.

```toml
[notifications.highlight.exclude]
criteria = [{ channel = "#halloy", user = "GH-Bot" }, 
            { channel = "#halloy", user = "ChanServ" }]
```

This example excludes messages in `#halloy` on the `libera` server only.

```toml
exclude = { criteria = [{ channel = "#halloy", server = "libera" }] }
```

This example excludes messages in all servers except `libera`.

```toml
include = { servers = "libera" }
exclude = "all"
```
