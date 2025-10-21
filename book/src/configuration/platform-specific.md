# `[platform_specific]`

Platform specific settings for Halloy.

## `macos`

macOS specific settings

### `content_padding`

Controls if the content (panes) are rendered inside the the titlebar content view, or padded just below it.

```toml
# Type: string
# Values: "embedded-content", "padded-content"
# Default: "embedded-content"

[platform_specific]
macos.content_padding = "embedded-content"
```

### `sidebar_padding`

Controls if the sidebar is rendered inside the the titlebar content view, or padded just below it.

```toml
# Type: string
# Values: "embedded-content", "padded-content"
# Default: "embedded-content"

[platform_specific]
macos.sidebar_padding = "embedded-content"
```

### `decorations`

Whether the window should have a border, traffic light, a title bar, etc. or not.

> ⚠️  You need a third party application to move the window around if set to false. Eg: Rectangle.app.

```toml
# Type: boolean
# Values: true, false
# Default: true

[platform_specific]
macos.decorations = false
```

## `linux`

Linux specific settings

### `decorations`

Whether the window should have a border, a title bar, etc. or not.

```toml
# Type: boolean
# Values: true, false
# Default: true

[platform_specific]
linux.decorations = false
```

## `windows`

Windows specific settings

### `decorations`

Whether the window should have a border, a title bar, etc. or not.

```toml
# Type: boolean
# Values: true, false
# Default: true

[platform_specific]
windows.decorations = false
```

