# `[sidebar]`

Sidebar settings for Halloy.

## `buffer_action`

Action when pressing buffers in the sidebar. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the pressed buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
buffer_action = "replace-pane"
```


## `buffer_focused_action`

Action when pressing a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

```toml
# Type: string
# Values: "close-pane"
# Default: not set

[sidebar]
buffer_focused_action = "close-pane"
```

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

## `width`

Specify sidebar width in pixels. Only used if `position` is `"left"` or `"right"`.

```toml
# Type: integer
# Values: any positive integer
# Default: 120

[sidebar]
width = 120
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