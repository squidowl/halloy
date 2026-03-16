# Sidebar

Sidebar settings for Halloy.

## server_icon

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

## position

Sidebar position within the application window.

```toml
# Type: string
# Values: "left", "top", "right", "bottom"
# Default: "left"

[sidebar]
position = "left"
```

## max_width

Specify sidebar max width in pixels. Only used if `position` is `"left"` or `"right"`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[sidebar]
max_width = 200
```

## user_menu

Show or hide the user menu button in the sidebar.

```toml
# Type: bool
# Values: true, false
# Default: true

[sidebar.user_menu]
enabled = true
```

## order_by

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

## order_channels_by

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

## channel_name_casing

Transform the channel name casing in the sidebar channel entries.

```toml
# Type: string (optional)
# Values: "lowercase"
# Default: not set (channel name displayed as-is)

[sidebar]
channel_name_casing = "lowercase"
```
