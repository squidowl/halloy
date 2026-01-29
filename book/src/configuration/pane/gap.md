# Gap

Gap configuration for pane spacing and padding.

- [Gap](#gap)
  - [Example](#example)
  - [Configuration](#configuration)
    - [inner](#inner)
    - [outer](#outer)

## Example

```toml
[pane]
inner = 4
outer = 4
```

## Configuration

### inner

Controls the spacing between panes in a pane grid.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 4

[pane.gap]
inner = 4
```

### outer

Controls the padding around the outer edge of the pane grid.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 8

[pane.gap]
outer = 8
```
