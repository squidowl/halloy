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