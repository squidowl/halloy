# Nickname

Customize how nicknames are displayed within a buffer.

- [Nickname](#nickname)
  - [Configuration](#configuration)
    - [alignment](#alignment)
    - [away](#away)
    - [brackets](#brackets)
    - [color](#color)
    - [offline](#offline)
    - [show\_access\_levels](#show_access_levels)
    - [shown\_status](#shown_status)
    - [click](#click)
    - [truncate](#truncate)
  - [Hide Consecutive](#hide-consecutive)

## Configuration

### alignment

Horizontal alignment of nicknames.

```toml
# Type: string
# Values: "left", "right", "top"
# Default: "left"

[buffer.nickname]
alignment = "right"
```

### away

Controls the appearance of away nicknames.

```toml
# Type: string or object
# Values: "dimmed", "none" or { dimmed = float }
# Default: "dimmed"
[buffer.nickname]
away = "dimmed"

# with custom dimming alpha value (0.0-1.0)
[buffer.nickname]
away = { dimmed = 0.5 }

# no away indication
[buffer.nickname]
away = "none"
```

### brackets

Brackets around nicknames.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.nickname]
brackets = { left = "<", right = ">" }
```

### color

Nickname colors in a channel buffer. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.nickname]
color = "unique"
```

### offline

Controls the appearance of offline nicknames.

```toml
# Type: string or object
# Values: "solid" or "none"
# Default: "solid"
[buffer.nickname]
offline = "solid"

# no offline indication
[buffer.nickname]
offline = "none"
```

### show_access_levels

Show access level(s) in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: string
# Values: "all", "highest", or "none"
# Default: "highest"

[buffer.nickname]
show_access_levels = "none"
```

### shown_status

What status should be indicated (by either `away` or `offline` settings), the user's current status (`"current"`) or their status at the time of sending the message (`"historical"`).

```toml
# Type: string or object
# Values: "current" or "historical"
# Default: "current"
[buffer.nickname]
shown_status = "current"
```

### click

Click action for when interaction with nicknames.

- `"open-query"`: Open a query with the User
- `"insert-nickname"`: Inserts the nickname into text input

```toml
# Type: string
# Values: "open-query", "insert-nickname"
# Default: "open-query"

[buffer.nickname]
click = "open-query"
```

### truncate

Truncate nicknames in buffer to a maximum length

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.nickname]
truncate = 10
```

## [Hide Consecutive](hide-consecutive.md)

Hide nickname if consecutive messages are from the same user.
