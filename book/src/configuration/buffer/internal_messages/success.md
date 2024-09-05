# `[buffer.internal_messages.success]`

 A internal messages which is considered a "success" such as when a connection was restored, or when connected succesfully to a server.

**Example**

```toml
[buffer.internal_messages.success]
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
