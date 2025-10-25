# Sidebar

How sidebar actions should be enacted.

- [Sidebar](#sidebar)
  - [Example](#example)
  - [Configuration](#configuration)
    - [buffer](#buffer)
    - [focused\_buffer](#focused_buffer)

## Example

```toml
# Open a new pane when clicking on a buffer in the sidebar
# (or close the buffer if it's already open)

[actions.sidebar]
buffer = "new-pane"
focused_buffer = "close-pane"
```

## Configuration

### buffer

Action when clicking buffers in the sidebar. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.sidebar]
buffer = "replace-pane"
```

### focused_buffer

Action when clicking a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

```toml
# Type: string
# Values: "close-pane"
# Default: not set

[actions.sidebar]
focused_buffer = "close-pane"
```
