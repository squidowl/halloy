# `[sidebar]`

Customize the sidebar of Halloy.

**Example**

```toml
[sidebar]
buffer_action = "replace-pane"
buffer_focused_action = "close-pane"
unread_indicators = "title"
position = "top"
```

## `buffer_action`

Action when pressing buffers in the sidebar. `"new-pane"` opens a new pane each time. `"repace-pane"` replaces the focused pane with the pressed buffer. `"new-window"` opens a new window each time.

- **type**: string
- **values**: `"new-pane"`, `"replace-pane"`, `"new-window"`
- **default**: `"new-pane"`

## `buffer_focused_action`

Action when pressing a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

- **type**: string
- **values**: `"close-pane"`
- **default**: not set

## `unread_indicators`

Unread buffer indicator style.

- **type**: string
- **values**: `"dot"`, `"title"`, `"none"`
- **default**: `"dot"`

## `position`

Sidebar position within the application window.

- **type**: string
- **values**: `"left"`, `"top"`, `"right"`, `"bottom"`
- **default**: `"left"`

## `width`

Specify sidebar width in pixels. Only used if `position` is `"left"` or `"right"`

- **type**: integer
- **values**: any positive integer
- **default**: `120"`
