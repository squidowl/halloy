# Error

Internal messages which are considered an "error" such as when a connection was lost, or when connection to server failed.

- [Error](#error)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [smart](#smart)

## Configuration

### enabled

Control if internal message type is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.internal_messages.error]
enabled = true
```

### smart

Only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.error]
smart = 180
```
