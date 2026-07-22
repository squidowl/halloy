# Sidebar

Sidebar settings for Halloy.

## `primary_font_size`

Configure the font size used for server and internal buffer titles.  If not set, then [`sidebar.secondary_font_size`](./sidebar#secondary_font_size) will be used.

```toml
# Type: integer
# Values: any positive integer or not set
# Default: not set

[sidebar]
primary_font_size = 12
```

## `primary_icon`

Configure the icon display for servers and internal buffers.

```toml
# Type: integer or string
# Values: any positive integer or "hidden"
# Default: 12

[sidebar]
primary_icon = 12
```

Hide server and internal buffer icons:

```toml
[sidebar]
primary_icon = "hidden"
```

## `secondary_font_size`

Configure the font size used for buffers in the sidebar.  If not set, then [`font.size`](./font#size) will be used.

```toml
# Type: integer
# Values: any positive integer or not set
# Default: not set

[sidebar]
secondary_font_size = 12
```

## `position`

Sidebar position within the application window.

```toml
# Type: string
# Values: "left", "top", "right", "bottom"
# Default: "left"

[sidebar]
position = "left"
```

## `max_width`

Specify sidebar max width in pixels. Only used if `position` is `"left"` or `"right"`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[sidebar]
max_width = 200
```

## `user_menu`

Show or hide the user menu button in the sidebar.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar.user_menu]
enabled = true
```

## `collapse_button`

Show or hide the button used to expand and collapse a server's channels and
queries. Servers can still be expanded or collapsed from their context menu
when this button is hidden.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar.collapse_button]
enabled = true
```

## `order_by`

Ordering that servers are listed in the sidebar.

- `"config"`: The same order they are specified in the configuration file.
- `"alpha"`: Case-insensitive alphabetical ordering.

```toml
# Type: string
# Values: "alpha", "config"
# Default: "alpha"

[sidebar]
order_by = "config"
```

## `order_channels_by`

Ordering for channels listed in the sidebar.

- `"name"`: Sort channels by name only, ignoring chantypes (channel prefixes, e.g., `#` and `##`).
- `"name-and-prefix"`: Sort channels by name including their chantypes.
- `"config"`: Sort channels in the order they appear in your server's `channels`
  list. Any channels not in the list appear last, using default (`"name"`) sort.

```toml
# Type: string
# Values: "name", "name-and-prefix", "config"
# Default: "name"

[sidebar]
order_channels_by = "config"

# Example: When using "config", channels appear in this exact order:
[servers.liberachat]
channels = ["#rust", "#halloy", "#halloy-test"]
# Result: #rust → #halloy → #halloy-test → (any other channels are sorted by "name")
```

## `internal_buffers` {#internal-buffers}

Configure which internal buffers appear in the sidebar and whether they are
placed before or after IRC servers.

```toml
# Type: table
# Values: `position`, `buffers`, and `mute`
# Default: `{ position = "after-servers", buffers = [] }`

[sidebar.internal_buffers]
position = "after-servers"
buffers = ["logs", "highlights"]
```

### `position`

Controls whether internal buffers appear before or after IRC servers in the sidebar.

```toml
# Type: string
# Values: "before-servers", "after-servers"
# Default: "after-servers"

[sidebar.internal_buffers]
position = "before-servers"
```

### `buffers`

Internal buffers shown in the sidebar.

```toml
# Type: array of strings
# Values: "channel-discovery", "channel-monitor", "config-editor", "file-transfers", "highlights", "logs"
# Default: []

[sidebar.internal_buffers]
buffers = ["logs", "highlights"]
```

### `mute`

Controls when the internal buffers are muted (hidden from the sidebar).

- `"never"`: Never mute the internal buffers (i.e. internal buffers are always visible).
- `"read"`: Mute internal buffers if they have no unread messages (i.e. only show internal buffers with unread messages).

Note: If a buffer has no concept of "unread messages" (e.g., `file-transfer`), then it will be shown in the sidebar regardless of this setting.

```toml
# Type: string
# Values: "never", "read"
# Default: "never"

[sidebar.internal_buffers]
mute = "read"
```

## `channel_name_casing`

Transform the channel name casing in the sidebar channel entries.

```toml
# Type: string (optional)
# Values: "lowercase"
# Default: not set (channel name displayed as-is)

[sidebar]
channel_name_casing = "lowercase"
```

## `scrollbar`

Scrollbar in sidebar

### `width`

Width of the scrollbar.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[sidebar.scrollbar]
width = 5
```

### `scroller_width`

Width of the scrollbar scroller.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[sidebar.scrollbar]
scroller_width = 5
```

## `unread_indicator`

Unread message in buffer indicator style.

### `title`

Changes buffer title color when unread messages are present

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar.unread_indicator]
title = false
```

### `icon`

Changes the icon which appears when unread messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "dot"

[sidebar.unread_indicator]
icon = "dot"
```

### `icon_size`

Changes the unread message icon size.

```toml
# Type: integer
# Values: any positive integer"
# Default: 6

[sidebar.unread_indicator]
icon_size = 6
```

### `show_on_open_buffers`

Show unread message indicators on buffers that have an open pane.

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.unread_indicator]
show_on_open_buffers = false
```


### `query_as_highlight`

Treat unread messages in query (direct message) buffers as highlights for
sidebar styling. When enabled, the resulting indicator for messages will be
controlled by [`sidebar.highlight_indicator`](#highlight-indicator) settings.

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar.unread_indicator]
query_as_highlight = true
```

### `exclude`

[Exclusion conditions](/configuration/conditions.md) for which unread indicators
won't be shown. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.
Does not suppress unread highlight indicators; use
[`highlight_indicator.exclude`](#exclude-1) to suppress unread highlight
indicators. To hide both unread message and highlight indicators for the same
buffers, set the same conditions on `unread_indicator.exclude` and
`highlight_indicator.exclude`.

```toml
# Type: inclusion/exclusion conditions
# Values: channel, user, & server inclusion/exclusion conditions
# Default: not set

[sidebar.unread_indicator]
exclude = { channels = ["#noisy-channel"] }
```

### `include`

[Inclusion conditions](/configuration/conditions.md) for which unread indicators
will be shown. Unread indicators are enabled in all conditions unless explicitly
excluded, so this setting is only relevant when combined with the `exclude`
setting.

```toml
# Type: inclusion/exclusion conditions
# Values: channel, user, & server inclusion/exclusion conditions
# Default: not set

[sidebar.unread_indicator]
exclude = "*"
include = { channels = ["#halloy"] }
```

## `highlight_indicator` {#highlight-indicator}

Unread highlight in buffer indicator style.

### `title`

Changes buffer title color when unread highlights are present

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar.highlight_indicator]
title = false
```

### `icon`

Changes the icon which appears when unread highlights are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "dot"

[sidebar.highlight_indicator]
icon = "dot"
```

### `icon_size`

Changes the unread highlight icon size.

```toml
# Type: integer
# Values: any positive integer"
# Default: 6

[sidebar.highlight_indicator]
icon_size = 6
```

### `show_on_open_buffers`

Show unread/highlight indicators on buffers that have an open pane.

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.highlight_indicator]
show_on_open_buffers = false
```

### `exclude`

[Exclusion conditions](/configuration/conditions.md) for which unread indicators
won't be shown. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.
Does not suppress unread message indicators; use
[`unread_indicator.exclude`](#exclude) to suppress unread message indicators. To
hide both unread message and highlight indicators for the same buffers, set the
same conditions on `unread_indicator.exclude` and `highlight_indicator.exclude`.

```toml
# Type: inclusion/exclusion conditions
# Values: channel, user, & server inclusion/exclusion conditions
# Default: not set

[sidebar.highlight_indicator]
exclude = { channels = ["#noisy-channel"] }
```

### `include`

[Inclusion conditions](/configuration/conditions.md) for which unread indicators
will be shown. Unread indicators are enabled in all conditions unless explicitly
excluded, so this setting is only relevant when combined with the `exclude`
setting.

```toml
# Type: inclusion/exclusion conditions
# Values: channel, user, & server inclusion/exclusion conditions
# Default: not set

[sidebar.highlight_indicator]
exclude = "*"
include = { channels = ["#halloy"] }
```

## `user_menu`

User menu in sidebar settings.

### `enabled`

Controls whether the user menu is shown in the sidebar or hidden

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.user_menu]
enabled = true
```

## `padding`

Adjust padding for sidebar

### `buffer`

Controls padding for buffer buttons (server, channels, queries) in the sidebar
The value is an array where the first value is vertical padding and the second is horizontal padding. 

```toml
# Type: array
# Values: array
# Default: [2, 2]

[sidebar.padding]
buffer = [2, 5]
```

## `spacing`

Adjust spacing for sidebar

### `server`

Controls the vertical spacing between servers (i.e. between the last buffer for one server and the server buffer for the next server).

```toml
# Type: integer
# Values: any non-negative integer
# Default: 6

[sidebar.spacing]
server = 4
```
