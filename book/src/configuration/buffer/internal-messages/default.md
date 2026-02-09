# Default

Default settings which will be used for all internal messages when a specific value is not provided for the specific internal message type.

- [Default](#default)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [smart](#smart)

## Configuration

### enabled

Control if internal messages are enabled by default.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.internal_messages.default]
enabled = true
```

### smart

By default, only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.default]
smart = 180
```
