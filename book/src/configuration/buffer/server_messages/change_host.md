# `[buffer.server_messages.change_host]`

Server message is sent when a user's host changes.

**Example**

```toml
[buffer.server_messages.change_host]
enabled = true
smart = 180
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
