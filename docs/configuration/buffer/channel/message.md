# Message

Message settings within a channel buffer.

## nickname_color

Nickname colors in the message. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.channel.message]
nickname_color = "unique"
```

## show_emoji_reacts

Whether to display emoji reactions on messages (if [IRCv3 React](https://ircv3.net/specs/client-tags/react) is supported by the server).

```toml
# Type: boolean
# Values: "true", "false"
# Default: "true"

[buffer.channel.message]
show_emoji_reacts = true
```

## max_reaction_display

Maximum number of user-visible characters (Unicode grapheme clusters) in a reaction.
If a reaction exceeds this value, then its display is truncated to the first `max_reaction_display` grapheme clusters.

```toml
# Type: integer
# Values: positive integers
# Default: 5

[buffer.channel.message]
max_reaction_display = 5
```

## max_reaction_chars

Maximum number of user-visible characters (Unicode grapheme clusters) in a reaction.
If a reaction exceeds this value, then it is not stored.

```toml
# Type: integer
# Values: positive integers
# Default: 64

[buffer.channel.message]
max_reaction_chars = 64
```
