# Spacing

Spacing settings for Halloy.

This section provides aliases for spacing-related options that also exist under other sections (e.g. `[buffer]`, `[pane]`, `[sidebar]`, `[context_menu]`). Values set here take precedence when both are present.

## Configuration

```toml
[spacing.buffer]
# Alias of: [buffer] line_spacing
line_spacing = 4

[spacing.pane.gap]
# Alias of: [pane.gap] inner / outer
inner = 6
outer = 10

[spacing.sidebar.padding]
# Alias of: [sidebar.padding] buffer
buffer = [5, 5]

[spacing.sidebar.spacing]
# Alias of: [sidebar.spacing] server
server = 12

[spacing.context_menu.padding]
# Alias of: [context_menu.padding] entry
entry = [2, 5]
```
