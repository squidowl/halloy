# `[buffer.channel.message]`

Customize the message within a channel buffer.

**Example**

```toml
[buffer.channel.message]
nickname_color = "unique"
```

## `nickname_color`
Nickname colors in the message. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

- **type**: string
- **values**: `"solid"`, `"unique"`
- **default**: `"unique"`