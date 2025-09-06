# `[sidebar]`

Sidebar settings for Halloy.

## `server_icon_size`

Adjust server icon size

```toml
# Type: integer
# Values: any positive integer"
# Default: 12

[sidebar]
server_icon_size = 12
```

## `position`

Sidebar position within the application window.

```toml
# Type: string
# Values: "left", "top", "right", "bottom"
# Default: "left"

[sidebar]
position = "left"
```

## `max_width`

Specify sidebar max width in pixels. Only used if `position` is `"left"` or `"right"`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[sidebar]
max_width = 200
```

## `show_menu_button`

Show or hide the user menu button in the sidebar.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar]
show_menu_button = true
```

## `order_by`

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

## `scrollbar`

Scrollbar configuration.

### width

Width of the scrollbar.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[sidebar.scrollbar]
width = 5
```

### width

Width of the scrollbar scroller.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[sidebar.scrollbar]
scroller_width = 5
```

## `unread_indicator`

Unread buffer indicator style.

### `title`

Changes buffer title color when unread messages are present

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar.unread_indicator]
title = false
```

### `icon`

Changes the icon which appears when unread messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "dot"

[sidebar.unread_indicator]
icon = "dot"
```

### `highlight_icon`

Changes the icon which appears when unread highlight messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "dot"

[sidebar.unread_indicator]
highlight_icon = "circle-empty"
```

### `icon_size`

Changes the unread icon size.

```toml
# Type: integer
# Values: any positive integer"
# Default: 6

[sidebar.unread_indicator]
icon_size = 6
```

### `highlight_icon_size`

Changes the highlight unread icon size.

```toml
# Type: integer
# Values: any positive integer"
# Default: 8

[sidebar.unread_indicator]
highlight_icon_size = 8
```

