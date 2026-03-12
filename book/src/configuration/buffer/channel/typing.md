# Typing

Typing settings for channel and query buffers.

- [Typing](#typing)
  - [Configuration](#configuration)
    - [font_size](#font_size)
    - [share](#share)
    - [show](#show)

## Configuration

### font_size

Control the font size of the typing indicator. This also adjusts the bottom padding reserved for the typing indicator line.

```toml
# Type: integer
# Values: positive integers
# Default: not set
# When omitted, Halloy uses the main configured font size.

[buffer.channel.typing]
font_size = 12
```

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
