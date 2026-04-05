# Sidebar

Sidebar settings for Halloy.

## `server_icon`

Configure the server icon display.

```toml
# Type: integer or string
# Values: any positive integer or "hidden"
# Default: 12

[sidebar]
server_icon = 12
```

Hide the server icon:

```toml
[sidebar]
server_icon = "hidden"
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

Unread buffer indicator style.

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

### `highlight_icon`

Changes the icon which appears when unread highlight messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "circle-empty"

[sidebar.unread_indicator]
highlight_icon = "circle-empty"
```

### `icon_size`

Changes the unread icon size.

Note: If set larger than the line height of the specified [font](/configuration/font) then the icon will not render.

```toml
# Type: integer
# Values: any positive integer"
# Default: 6

[sidebar.unread_indicator]
icon_size = 6
```

### `highlight_icon_size`

Changes the highlight unread icon size.

Note: If set larger than the line height of the specified [font](/configuration/font) then the icon will not render.

```toml
# Type: integer
# Values: any positive integer"
# Default: 8

[sidebar.unread_indicator]
highlight_icon_size = 8
```

### `show_on_open_buffers`

Show unread/highlight indicators on buffers that have an open pane.

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.show_on_open_buffers]
show_on_open_buffers = true
```


### `query_as_highlight`

Treat unread query (direct message) buffers as highlights for sidebar styling.

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
