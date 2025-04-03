# `[sidebar]`

Sidebar settings for Halloy.

## `unread_indicator`

Unread buffer indicator style.

```toml
# Type: string
# Values: "dot", "title", "none"
# Default: "dot"

[sidebar]
unread_indicator = "dot"
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
# Values: any positive integer
# Default: not set

[sidebar]
max_width = 200
```

## `show_menu_button`

Show or hide the user menu button in the sidemenu.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar]
show_menu_button = true
```
