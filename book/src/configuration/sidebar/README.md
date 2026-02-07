# Sidebar

Sidebar settings for Halloy.

- [Sidebar](#sidebar)
  - [Configuration](#configuration)
    - [server\_icon](#server_icon)
    - [position](#position)
    - [max\_width](#max_width)
    - [show\_nicklist](#show_nicklist)
    - [split](#split)
    - [buflist\_space](#buflist_space)
    - [nicklist\_space](#nicklist_space)
    - [show\_menu\_button](#show_menu_button)
    - [order\_by](#order_by)
  - [Scrollbar](#scrollbar)
  - [Unread Indicator](#unread-indicator)
  - [Padding](#padding)
  - [Spacing](#spacing)

## Configuration

### server_icon

Configure the server icon display.

```toml
# Type: integer or string
# Values: any positive integer or "hidden"
# Default: 12

[sidebar]
server_icon = 12
```

Hide the server icon:

```toml
[sidebar]
server_icon = "hidden"
```

### position

Sidebar position within the application window.

```toml
# Type: string
# Values: "left", "top", "right", "bottom"
# Default: "left"

[sidebar]
position = "left"
```

### max_width

Specify sidebar max width in pixels. Only used if `position` is `"left"` or `"right"`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[sidebar]
max_width = 200
```

### show_nicklist

Show the nicklist for the focused channel in the sidebar (rendered below the buffer list). Only used if `position` is `"left"` or `"right"`.

```toml
# Type: bool
# Values: true, false
# Default: false

[sidebar]
show_nicklist = true
```

### split

When `show_nicklist` is enabled, controls how the vertical space is split between the buffer list and the nicklist.

- `true`: buffer list and nicklist use proportional space controlled by `buflist_space` and `nicklist_space`.
- `false`: buffer list shrinks; nicklist takes the remaining space.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar]
split = false
```

### buflist_space

Relative space for the buffer list when `split = true`.

```toml
# Type: integer
# Values: any positive integer
# Default: 2

[sidebar]
buflist_space = 2
```

### nicklist_space

Relative space for the nicklist when `split = true`.

```toml
# Type: integer
# Values: any positive integer
# Default: 1

[sidebar]
nicklist_space = 1
```

### show_menu_button

Show or hide the user menu button in the sidebar.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar]
show_menu_button = true
```

### order_by

Ordering that servers are listed in the sidebar uses to select from matching users.

- `"config"`: The same order they are specified in the configuration file.
- `"alpha"`: Case-insensitive alphabetical ordering.

```toml
# Type: string
# Values: "alpha", "config"
# Default: "alpha"

[sidebar]
order_by = "config"
```

## [Scrollbar](scrollbar.md)

Scrollbar in sidebar

## [Unread Indicator](unread-indicator.md)

Unread buffer indicator style

## [Padding](padding.md)

Adjust padding for sidebar

## [Spacing](spacing.md)

Adjust spacing for sidebar
