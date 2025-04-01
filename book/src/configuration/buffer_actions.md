# `[buffer_actions]`

Application wide buffer actions (that open or close buffers).

**Example**

```toml
# Replace pane when clicking on channel/user names in a pane,
# open a new pane when clicking on a buffer in the sidebar
# (or close the buffer if it's already open)
[buffer_actions]
click_buffer = "new-pane"
click_focused_buffer = "close-pane"
click_channel_name = "replace-pane"
click_user_name = "replace-pane"
```

## `click_buffer`

Action when clicking buffers in the sidebar. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
click_buffer = "replace-pane"
```

## `click_focused_buffer`

Action when clicking a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

```toml
# Type: string
# Values: "close-pane"
# Default: not set

[sidebar]
click_focused_buffer = "close-pane"
```

## `click_channel_name`

Action when clicking on a channel name in a pane. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
click_channel_name = "new-pane"
```

## `click_highlight`

Action when clicking on a highlight in the highlights buffer. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the buffer that contains the highlight. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
click_highlight = "new-pane"
```

## `click_user_name`

Action when clicking on a user name in a pane. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for clicked user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
click_user_name = "new-pane"
```

## `local_buffer`

Action when opening a local buffer (the highlights or logs buffer). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the local buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
local_buffer = "new-pane"
```

## `message_channel`

Action when sending an empty message to a channel (via the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
message_channel = "replace-pane"
```

## `message_user`

Action when sending an empty message to a user (via `Message` in the user context menu or the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for the user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
message_user = "replace-pane"
```
