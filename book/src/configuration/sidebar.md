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