# Message

Message settings within a channel buffer.

- [Message](#message)
  - [Configuration](#configuration)
    - [nickname\_color](#nickname_color)
    - [show\_emoji\_reacts](#show_emoji_reacts)
    - [max\_reaction\_chars](#max_reaction_chars)

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

Whether to display emoji reactions on messages (if [IRCv3 React](https://ircv3.net/specs/client-tags/react) is supported by the server).

```toml
# Type: boolean
# Values: "true", "false"
# Default: "true"

[buffer.channel.message]
show_emoji_reacts = true
```

### max_reaction_chars

Maximum number of user-visible characters (Unicode grapheme clusters) in a reaction.
If a reaction exceeds this value, it is truncated to the first `max_reaction_chars` grapheme clusters.

```toml
# Type: integer
# Values: positive integers
# Default: 5

[buffer.channel.message]
max_reaction_chars = 5
```
