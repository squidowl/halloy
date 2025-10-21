# Sidebar

Sidebar settings for Halloy.

- [Sidebar](#sidebar)
  - [Configuration](#configuration)
    - [server\_icon\_size](#server_icon_size)
    - [position](#position)
    - [max\_width](#max_width)
    - [show\_menu\_button](#show_menu_button)
    - [order\_by](#order_by)
  - [Sub-sections](#sub-sections)
    - [Scrollbar](#scrollbar)
    - [Unread Indicator](#unread-indicator)

## Configuration

### server_icon_size

Adjust server icon size.

Note: If set larger than the line height of the specified [font](../font/) then the icon will not render.

```toml
# Type: integer
# Values: any positive integer"
# Default: 12

[sidebar]
server_icon_size = 12
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

## Sub-sections

### [Scrollbar](scrollbar.md)

Scrollbar in sidebar

### [Unread Indicator](unread-indicator.md)

Unread buffer indicator style
