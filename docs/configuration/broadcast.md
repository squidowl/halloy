# Broadcast

Message broadcast settings for Halloy.

## `reconnected`

Whether to broadcast a message to channels and queries when a connection is re-established after being lost.

```toml
# Type: boolean
# Values: true, false
# Default: true

[broadcast]
reconnected = true
```

## `disconnected`

Whether to broadcast a message to channels and queries when a connection is lost.

```toml# Type: boolean
# Values: true, false
# Default: true
[broadcast]
disconnected = true
```
