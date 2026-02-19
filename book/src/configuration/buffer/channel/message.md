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

### show_emoji_reacts

Whether to display emoji reactions on messages.

```toml
# Type: boolean
# Values: "true", "false"
# Default: "true"

[buffer.channel.message]
show_emoji_reacts = true
```
