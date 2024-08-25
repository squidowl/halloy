# Sidebar

## `[sidebar]` Section

```toml
[sidebar]
default_action = "new-pane" | "replace-pane"
unread_indicators = "dot" | "title" | "none"
position = "left" | "right" | "top" | "bottom"
width = <integer>
```

| Key                 | Description                                                                            | Default      |
| ------------------- | -------------------------------------------------------------------------------------- | ------------ |
| `default_action`    | Action when selecting buffers in the sidebar. Can be `"new-pane"` or `"replace-pane"`. | `"new-pane"` |
| `unread_indicators` | Unread buffer indicator style.                                                         | `"dot"`      |
| `position`          | Sidebar position.                                                                      | `"left"`     |
| `width`             | Specify sidebar width in pixels. Only used if `position` is `"left"` or `"right"`.     | `120`        |

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
