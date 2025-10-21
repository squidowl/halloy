# Message

Message settings within a channel buffer.

- [Message](#message)
  - [Configuration](#configuration)
    - [nickname\_color](#nickname_color)

## Configuration

### nickname_color

Nickname colors in the message. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.channel.message]
nickname_color = "unique"
```
