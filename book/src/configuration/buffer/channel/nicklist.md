# `[buffer.channel.nicklist]`

The `[buffer.channel.nicklist]` section allows you to customize the nicklist within a channel buffer.

**Example**

```toml
[buffer.channel.nicklist]
color = "unique"
show_access_levels = true
```

## `alignment`

Horizontal alignment of nicknames.

- **type**: string
- **values**: `"left"`, `"right"`
- **default**: `"left"`

## `color`
Nickname colors in the nicklist. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

- **type**: string
- **values**: `"solid"`, `"unique"`
- **default**: `"unique"`

## `enabled`

Control if nicklist should be shown or not by default.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `position`

Nicklist position in the buffer.

- **type**: string
- **values**: `"left"`, `"right"`
- **default**: `"right"`


## `show_access_levels`

Show access levels in front of nicknames (`@`, `+`, `~`, etc.).

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `width`

Overwrite nicklist width in pixels.

- **type**: integer
- **values**: any positive integer
- **default**: not set
