# Typing

Typing settings for channel and query buffers.

- [Typing](#typing)
  - [Configuration](#configuration)
    - [share](#share)
    - [show](#show)

## Configuration

### share

Control whether Halloy shares your typing status with other users.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.channel.typing]
share = false
```

### show

Control whether Halloy shows typing status from other users.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.typing]
show = true
```
