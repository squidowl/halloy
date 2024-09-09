# `[buffer.server_messages.topic]`

Server message is sent when a (op) user changes channel topic.

**Example**

```toml
[buffer.server_messages.topic]
enabled = true
```

## `enabled`

Control if internal message type is enabled.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

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