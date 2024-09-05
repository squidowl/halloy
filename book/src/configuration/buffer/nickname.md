# `[buffer.nickname]`

Customize how nicknames are displayed within a buffer.

**Example**

```toml
[buffer.nickname]
alignment = "right" 
brackets = { left = "<", right = ">" }
color = "unique"
show_access_levels = true
```

## `alignment`

Horizontal alignment of nicknames.

- **type**: string
- **values**: `"left"`, `"right"`
- **default**: `"left"`

## `brackets`

Brackets around nicknames. 

- **type**: object
- **values**: `{ left = "<any string>", right = "<any string>" }`
- **default**: `{ left = "", right = "" }`

## `color`
Nickname colors in a channel buffer. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

- **type**: string
- **values**: `"solid"`, `"unique"`
- **default**: `"unique"`

## `show_access_levels`

Show access levels in front of nicknames (`@`, `+`, `~`, etc.).

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`