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

## `exclude`

Exclude channels from receiving the server messag.
If you pass `["#halloy"]`, the channel `#halloy` will not receive the server message. You can also exclude all channels by using a wildcard: `["*"]`.

- **type**: array of strings
- **values**: array of any strings
- **default**: `[]`

## `include`

Include channels to receive the server message.
If you pass `["#halloy"]`, the channel `#halloy` will receive the server message. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

- **type**: array of strings
- **values**: array of any strings
- **default**: `[]`
