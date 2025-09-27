# `[pane]`

Pane settings for Halloy. A pane contains a [buffer](../configuration/buffer.md).

## `restore_on_launch`

Restore the panes that were open when Halloy was last closed when launching the application.

```toml
# Type: boolean
# Values: true, false
# Default: true

[pane]
restore_on_launch = false
```

## `scrollbar`

Scrollbar configuration.

### width

Width of the scrollbar.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[pane.scrollbar]
width = 5
```

### width

Width of the scrollbar scroller.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[pane.scrollbar]
scroller_width = 5
```

## `split_axis`

Default axis used when splitting the focused pane to create a new pane (i.e. default orientation of the divider between panes).  `"shorter"` will compare the width and height of the pane to select the splitting axis;  if the width is shorter then the horizontal axis is selected, and if the height is hosrter then the vertical axis is selected.  `"largest-shorter"` will split the largest pane in the main window using the same method as `"shorter"`, rather than splitting the focused pane.

```toml
# Type: string
# Values: "horizontal", "largest-shorter", "shorter", "vertical"
# Default: "shorter"

[pane]
split_axis = "vertical"
```
