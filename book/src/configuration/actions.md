# `[actions]`

Application-wide actions;  how user actions should be enacted.

1. [Buffer](#actionsbuffer) - Buffer actions
2. [Sidebar](#actionssidebar) - Sidebar actions

**Example**

```toml
# Replace pane when clicking on channel/user names in a pane,
# open a new pane when clicking on a buffer in the sidebar
# (or close the buffer if it's already open)

[actions.sidebar]
buffer = "new-pane"
focused_buffer = "close-pane"

[actions.buffer]
click_channel_name = "replace-pane"
click_username = "replace-pane"
```

## `[actions.buffer]`

How buffer actions should be enacted.

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

## `click_username`

Action when clicking on a user name in a pane (if `buffer.channel.nicklist` or `buffer.nickname` is set to `"open-query"`). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for clicked user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
click_username = "new-pane"
```

## `local`

Action when opening a local buffer (the highlights or logs buffer). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the local buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
local = "new-pane"
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

## `[actions.sidebar]`

How sidebar actions should be enacted.

## `buffer`

Action when clicking buffers in the sidebar. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[sidebar]
buffer = "replace-pane"
```

## `focused_buffer`

Action when clicking a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

```toml
# Type: string
# Values: "close-pane"
# Default: not set

[sidebar]
focused_buffer = "close-pane"
```
