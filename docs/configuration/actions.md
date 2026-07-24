# Actions

Application-wide actions; how user actions should be enacted.

## `buffer`

How buffer actions should be enacted.

```toml
# Replace pane when clicking on channel/user names in a pane

[actions.buffer]
click_channel_name = "replace-pane"
click_nickname = "replace-pane"
```

### `click_channel_name`

Action when clicking on a channel name in a pane. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked channel. `"new-window"` opens a new window each time. `"no-action"` or `"noop"` will ignore clicks on channel names.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window", "no-action", "noop"
# Default: "new-pane"

[actions.buffer]
click_channel_name = "new-pane"
```

### `click_highlight`

Action when clicking on the channel name of a highlight in the highlights buffer. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the buffer that contains the highlight. `"new-window"` opens a new window each time. `"no-action"` or `"noop"` will ignore clicks on the channel name of highlights.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window", "no-action", "noop"
# Default: "new-pane"

[actions.buffer]
click_highlight = "new-pane"
```

### `click_channel_discovery`

Action when clicking on a channel name in the channel discovery pane. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked channel. `"new-window"` opens a new window each time. `"no-action"` or `"noop"` will ignore clicks on the channel names in the channel discovery pane.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window", "no-action", "noop"
# Default: "new-pane"

[actions.buffer]
click_channel_discovery = "new-pane"
```

### `click_nickname`

Click action for when interaction with nicknames.

- `"open-query" = "buffer-action"`: Open a query with the user with the prescribed buffer action (`"new-pane"`, `"replace-pane"`, or `"new-window"`). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for clicked user. `"new-window"` opens a new window each time.
- `"insert-nickname"`: Inserts the nickname into the buffer's input box.
- `"no-action"` or `"noop"`: No action is performed

```toml
# Type: string
# Values: "open-query" = "new-pane", "open-query" = "replace-pane", "open-query" = "new-window", "insert-nickname", "no-action", "noop"
# Default: "open-query" = "new-pane"

[actions.buffer]
click_nickname = { "open-query" = "replace-pane" }
```

### `open_internal`

Action when opening an internal buffer (e.g. highlights, logs, or channel discovery buffers) via keyboard shortcut, command bar, user menu, or slash-command (e.g. `/list`). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the internal buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
open_internal = "new-pane"
```

### `message_channel`

Action when sending an empty message to a channel (via the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
message_channel = "replace-pane"
```

### `message_user`

Action when sending an empty message to a user (via `Message` in the user context menu or the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for the user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
message_user = "replace-pane"
```

### `join_channel`

Action when joining a channel via `/join` command. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
join_channel = "replace-pane"
```

### `only_contract_expanded_message`

All actions on an expanded message will be disabled in favor of closing the message.  In other words, if set to `true` then clicking anywhere on an expanded message will contract it.

```toml
# Type: bool
# Values: true, false
# Default: true

[actions.buffer]
only_contract_expanded_message = false
```

### `click_image_url`

Click action when interacting with URLs of images in a message.

```toml
# Type: string
# Values: "open-url", "preview"
# Default: "open-url"

[actions.buffer]
click_image_url = "preview"
```

## `nicklist`

How nicklist actions should be enacted.

```toml
# Replace pane when clicking on a nickname in the nicklist

[actions.buffer]
click_nickname = "replace-pane"
```

### `click_nickname`

Click action for when interaction with nicknames.  If not set, then the behavior specified by [`actions.buffer.click_nickname`](#click_nickname) will be used.

- `"open-query" = "buffer-action"`: Open a query with the user with the prescribed buffer action (`"new-pane"`, `"replace-pane"`, or `"new-window"`). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for clicked user. `"new-window"` opens a new window each time.
- `"insert-nickname"`: Inserts the nickname into the buffer's input box.
- `"no-action"` or `"noop"`: No action is performed

```toml
# Type: string
# Values: "open-query" = "new-pane", "open-query" = "replace-pane", "open-query" = "new-window", "insert-nickname", "no-action", "noop", not set
# Default: not set

[actions.nicklist]
click_nickname = { "open-query" = "replace-pane" }
```

## `notification`

How notification actions should be enacted.

```toml
# Open buffer in a new window when clicking on a notification

[actions.notification]
default = "open-buffer"
open_buffer = "new-window"
```

### `default`

Default action when clicking on a notification.  When set to `"activate-application"` or when there is no buffer context for the notification (e.g. when clicking a notification for a monitored user going offline), then the default application activation behavior for notification will be enacted.  When there is a buffer context (e.g. when clicking a notification for a highlight in a channel, the context is the channel buffer) and set to `"open-buffer"`, then the buffer context will be opened.  If there is a buffer context and `"activate-application"` is set, then `"open-buffer"` will be provided as a secondary action (if supported by the notification system).  When opening a buffer, [`actions.notification.open_buffer`](#open_buffer) determines how it will be opened.

```toml
# Type: string
# Values: "activate-application", "open-buffer"
# Default: "activate-application"

[actions.notification]
default = "open-buffer"
```

### `open_buffer`

When opening a buffer from a notification, how it will be opened. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.notification]
open_buffer = "replace-pane"
```

## `sidebar`

How sidebar actions should be enacted.

```toml
# Open a new pane when clicking on a buffer in the sidebar
# (or close the buffer if it's already open)

[actions.sidebar]
buffer = "new-pane"
channel = "replace-pane"
query = "new-window"
focused_buffer = "close-pane"
```

### `buffer`

Action when clicking buffers in the sidebar. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.sidebar]
buffer = "replace-pane"
```

### `channel`

Action when clicking a channel buffer in the sidebar. If unset, it falls back to the value of `buffer`.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: not set (falls back to `buffer`)

[actions.sidebar]
channel = "replace-pane"
```

### `query`

Action when clicking a user/query buffer in the sidebar. If unset, it falls back to the value of `buffer`.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: not set (falls back to `buffer`)

[actions.sidebar]
query = "new-window"
```

### `focused_buffer`

Action when clicking a focused buffer in the sidebar. `"close-pane"` will close the focused pane.

```toml
# Type: string
# Values: "close-pane"
# Default: not set

[actions.sidebar]
focused_buffer = "close-pane"
```

### `cycle`

How to cycle between buffers when using [cycle shortcuts](/configuration/keyboard#actions).  If set to `"skip-collapsed"` then buffers that are hidden because the server has been [collapsed](/configuration/servers#sidebar-visibility) will be skipped when cycling to the next/previous (unread) buffer.  If set to `"into-collapsed"` then cycling to the next/previous (unread) buffer will include buffers hidden because the server has been [collapsed](/configuration/servers#sidebar-visibility) (collapsed buffers with an open pane will be shown in the sidebar).

```toml
# Type: string
# Values: "into-collapsed", "skip-collapsed"
# Default: "into-collapsed"

[actions.sidebar]
cycle = "skip-collapsed"
```
