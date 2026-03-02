# Sidebar

Sidebar settings for Halloy.

- [Sidebar](#sidebar)
  - [Configuration](#configuration)
    - [server\_icon](#server_icon)
    - [position](#position)
    - [max\_width](#max_width)
    - [user\_menu](#user_menu)
    - [order\_by](#order_by)
    - [order\_channels\_by](#order_channels_by)
    - [lowercase\_channels](#lowercase_channels)
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

### user_menu

Show or hide the user menu button in the sidebar.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar.user_menu]
enabled = true
```

### order_by

Ordering that servers are listed in the sidebar.

- `"config"`: The same order they are specified in the configuration file.
- `"alpha"`: Case-insensitive alphabetical ordering.

```toml
# Type: string
# Values: "alpha", "config"
# Default: "alpha"

[sidebar]
order_by = "config"
```

### order_channels_by

Include chantypes (channel prefixes, e.g., `#` and `##`) when sorting channels in the sidebar.

- `"name"`: Sort channels by name only, ignoring chantypes.
- `"name-and-prefix"`: Sort channels by name including their chantypes.

```toml
# Type: string
# Values: "name", "name-and-prefix"
# Default: "name"

[sidebar]
order_channels_by = "name-and-prefix"
```

### lowercase_channels

Render channel names in lowercase in the sidebar channel entries.

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar]
lowercase_channels = true
```

## [Scrollbar](scrollbar.md)

Scrollbar in sidebar

## [Unread Indicator](unread-indicator.md)

Unread buffer indicator style

## [Padding](padding.md)

Adjust padding for sidebar

## [Spacing](spacing.md)

Adjust spacing for sidebar
