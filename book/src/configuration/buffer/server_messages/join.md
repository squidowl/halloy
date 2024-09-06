# `[buffer.server_messages.join]`

Server message is sent when a user joins a channel.

**Example**

```toml
[buffer.server_messages.join]
enabled = true
smart = 180
username_format = "full"
```

## `enabled`

Control if internal message type is enabled.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `smart`

Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.

- **type**: integer
- **values**: any positive integer
- **default**: not set

## `username_format`

Adjust the amount of information displayed for a username in server messages. If you choose `"short"`, only the nickname will be shown. If you choose `"full"`, the nickname, username, and hostname (if available) will be displayed.

- **type**: string
- **values**: `"full"`, `"short"`
- **default**: `"full"`