# Display

Display settings for Halloy.

## `direction_arrows`

Customize the arrows used for directional messages such as join, part, quit, kick, and CTCP messages.

### `left`

Arrow shown for left-facing events.

```toml
# Type: string
# Values: any string
# Default: "←"

[display]
direction_arrows = { left = "<" }
```

### `right`

Arrow shown for right-facing events.

```toml
# Type: string
# Values: any string
# Default: "→"

[display]
direction_arrows = { right = ">" }
```
