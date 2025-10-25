# Buffer

How buffer actions should be enacted

- [Buffer](#buffer)
  - [Example](#example)
  - [Configuration](#configuration)
    - [click\_channel\_name](#click_channel_name)
    - [click\_highlight](#click_highlight)
    - [click\_username](#click_username)
    - [local](#local)
    - [message\_channel](#message_channel)
    - [message\_user](#message_user)

## Example

```toml
# Replace pane when clicking on channel/user names in a pane,

[actions.buffer]
click_channel_name = "replace-pane"
click_username = "replace-pane"
```


## Configuration

### click_channel_name

Action when clicking on a channel name in a pane. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the clicked channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
click_channel_name = "new-pane"
```

### click_highlight

Action when clicking on a highlight in the highlights buffer. `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the buffer that contains the highlight. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
click_highlight = "new-pane"
```

### click_username

Action when clicking on a user name in a pane (if `buffer.channel.nicklist` or `buffer.nickname` is set to `"open-query"`). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for clicked user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
click_username = "new-pane"
```

### local

Action when opening a local buffer (the highlights or logs buffer). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the local buffer. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
local = "new-pane"
```

### message_channel

Action when sending an empty message to a channel (via the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with the channel. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
message_channel = "replace-pane"
```

### message_user

Action when sending an empty message to a user (via `Message` in the user context menu or the `/msg` or `/notice` command). `"new-pane"` opens a new pane each time. `"replace-pane"` replaces the focused pane with a query for the user. `"new-window"` opens a new window each time.

```toml
# Type: string
# Values: "new-pane", "replace-pane", "new-window"
# Default: "new-pane"

[actions.buffer]
message_user = "replace-pane"
```
