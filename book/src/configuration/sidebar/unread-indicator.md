# Unread Indicator

Unread buffer indicator style.

- [Unread Indicator](#unread-indicator)
  - [Configuration](#configuration)
    - [title](#title)
    - [icon](#icon)
    - [highlight\_icon](#highlight_icon)
    - [icon\_size](#icon_size)
    - [highlight\_icon\_size](#highlight_icon_size)
    - [exclude](#exclude)
    - [include](#include)

## Configuration

### title

Changes buffer title color when unread messages are present

```toml
# Type: boolean
# Values: true, false
# Default: false

[sidebar.unread_indicator]
title = false
```

### icon

Changes the icon which appears when unread messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "dot"

[sidebar.unread_indicator]
icon = "dot"
```

### highlight_icon

Changes the icon which appears when unread highlight messages are present. To disable use `"none"`.

```toml
# Type: string
# Values: "dot", "circle-empty", "dot-circled", "certificate", "asterisk", "speaker", "lightbulb", "star", "none"
# Default: "circle-empty"

[sidebar.unread_indicator]
highlight_icon = "circle-empty"
```

### icon_size

Changes the unread icon size.

Note: If set larger than the line height of the specified [font](../font/) then the icon will not render.

```toml
# Type: integer
# Values: any positive integer"
# Default: 6

[sidebar.unread_indicator]
icon_size = 6
```

### highlight_icon_size

Changes the highlight unread icon size.

Note: If set larger than the line height of the specified [font](../font/) then the icon will not render.

```toml
# Type: integer
# Values: any positive integer"
# Default: 8

[sidebar.unread_indicator]
highlight_icon_size = 8
```

### exclude

[Exclusion conditions](/configuration/conditions.md) for which unread indicators
won't be shown. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: channel & server inclusion/exclusion conditions
# Default: not set

[sidebar.unread_indicator]
exclude = { channels = ["#noisy-channel"] }
```

### include

[Inclusion conditions](/configuration/conditions.md) for which unread indicators
will be shown. Unread indicators are enabled in all conditions unless explicitly
excluded, so this setting is only relevant when combined with the `exclude`
setting.

```toml
# Type: inclusion/exclusion conditions
# Values: channel & server inclusion/exclusion conditions
# Default: not set

[sidebar.unread_indicator]
exclude = "*"
include = { channels = ["#halloy"] }
```
