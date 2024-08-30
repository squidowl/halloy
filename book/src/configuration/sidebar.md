# Sidebar

## `[sidebar]` Section

```toml
[sidebar]
buffer_action = "new-pane" | "replace-pane"
buffer_focused_action = "close-pane"
unread_indicators = "dot" | "title" | "none"
position = "left" | "right" | "top" | "bottom"
width = <integer>
```

| Key                     | Description                                                                                                                | Default      |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------ |
| `buffer_action`         | Action when pressing buffers in the sidebar. `"new-pane"` opens a new pane, and `"replace-pane"` replaces the focused pane | `"new-pane"` |
| `buffer_focused_action` | Action when pressing a focused buffer in the sidebar. `"close-pane"` will close the focused pane.                          | `not set`    |
| `unread_indicators`     | Unread buffer indicator style.                                                                                             | `"dot"`      |
| `position`              | Sidebar position.                                                                                                          | `"left"`     |
| `width`                 | Specify sidebar width in pixels. Only used if `position` is `"left"` or `"right"`.                                         | `120`        |

## `[sidebar.buttons]` Section

```toml
[sidebar.buttons]
file_transfer = true | false
command_bar = true | false
reload_config = true | false
```

| Key             | Description                      | Default |
| --------------- | -------------------------------- | ------- |
| `file_transfer` | File transfer button in sidebar. | `true`  |
| `command_bar`   | Command bar button in sidebar.   | `true`  |
| `reload_config` | Reload config button in sidebar. | `true`  |
