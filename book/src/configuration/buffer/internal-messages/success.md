# Success

A internal messages which is considered a "success" such as when a connection was restored, or when connected successfully to a server.

- [Success](#success)
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

[buffer.internal_messages.success]
enabled = true
```

### smart

Only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.success]
smart = 180
```
