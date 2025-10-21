# Nicklist

Nicklist settings within a channel buffer.

- [Nicklist](#nicklist)
  - [Configuration](#configuration)
    - [alignment](#alignment)
    - [away](#away)
    - [color](#color)
    - [enabled](#enabled)
    - [position](#position)
    - [show\_access\_levels](#show_access_levels)
    - [width](#width)
    - [click](#click)

## Configuration

### alignment

Horizontal alignment of nicknames.

```toml
# Type: string
# Values: "left", "right"
# Default: "left"

[buffer.channel.nicklist]
alignment = "left"
```

### away

Controls the appearance of away nicknames.

```toml
# Type: string or object
# Values: "dimmed", "none" or { dimmed = float }
# Default: "dimmed"
[buffer.channel.nicklist]
away = "dimmed"

# with custom dimming alpha value (0.0-1.0)
[buffer.channel.nicklist]
away = { dimmed = 0.5 }

# no away indication
[buffer.channel.nicklist]
away = "none"
```

### color

Nickname colors in the nicklist. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.channel.nicklist]
color = "unique"
```

### enabled

Control if nicklist should be shown or not by default.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.nicklist]
enabled = true
```

### position

Nicklist position in the pane.

```toml
# Type: string
# Values: "left", "right"
# Default: "left"

[buffer.channel.nicklist]
position = "right"
```

### show_access_levels

Show access levels in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.nicklist]
show_access_levels = true
```

### width

Overwrite nicklist width in pixels.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.channel.nicklist]
width = 150
```

### click

Click action for when interaction with nicknames.

- `"open-query"`: Open a query with the User
- `"insert-nickname"`: Inserts the nickname into text input

```toml
# Type: string
# Values: "open-query", "insert-nickname"
# Default: "open-query"

[buffer.channel.nicklist]
click = "open-query"
```
