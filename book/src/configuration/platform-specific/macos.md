# macOS

macOS specific settings

- [macOS](#macos)
  - [Configuration](#configuration)
    - [content\_padding](#content_padding)
    - [sidebar\_padding](#sidebar_padding)
    - [decorations](#decorations)

## Configuration

### content_padding

Controls if the content (panes) are rendered inside the the titlebar content view, or padded just below it.

```toml
# Type: string
# Values: "embedded-content", "padded-content"
# Default: "embedded-content"

[platform_specific]
macos.content_padding = "embedded-content"
```

### sidebar_padding

Controls if the sidebar is rendered inside the the titlebar content view, or padded just below it.

```toml
# Type: string
# Values: "embedded-content", "padded-content"
# Default: "padded-content"

[platform_specific]
macos.sidebar_padding = "embedded-content"
```

### decorations

Whether the window should have a border, traffic light, a title bar, etc. or not.

> 💡 A restart is required for this change to take effect.

```toml
# Type: boolean
# Values: true, false
# Default: true

[platform_specific]
macos.decorations = false
```
