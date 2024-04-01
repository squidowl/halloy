# Sidebar

## `[sidebar]` Section

```toml
[sidebar]
default_action = "new-pane" | "replace-pane"
width = <integer>
```

| Key              | Description                                                                            | Default      |
| ---------------- | -------------------------------------------------------------------------------------- | ------------ |
| `default_action` | Action when selecting buffers in the sidebar. Can be `"new-pane"` or `"replace-pane"`. | `"new-pane"` |
| `width`          | Specify sidebar width in pixels.                                                       | `120`        |

## `[sidebar.buttons]` Section

```toml
[sidebar.buttons]
file_transfer = true | false
command_bar = true | false
```

| Key             | Description                      | Default |
| --------------- | -------------------------------- | ------- |
| `file_transfer` | File transfer button in sidebar. | `true`  |
| `command_bar`   | Command bar button in sidebar.   | `true`  |
