# `[buffer.internal_messages.error]`

A internal messages which is considered a "error" such as when a connection was lost, or when connection to server failed.

**Example**

```toml
[buffer.internal_messages.error]
enabled = true
smart = 180
```

## `enabled`

Control if internal message type is enabled.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `smart`

Only show internal message if received within the given time duration (seconds).

- **type**: integer
- **values**: any positive integer
- **default**: not set
