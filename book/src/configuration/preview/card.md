# Card

Specific card preview settings.

- [Card](#card)
  - [Example](#example)
  - [Configuration](#configuration)
    - [show\_image](#show_image)
    - [include](#include)
    - [exclude](#exclude)

## Example 

```toml
[preview.card]
exclude = ["*"] # hide card previews in all channels
include = ["#halloy"] # show card previews in #halloy
```

## Configuration

### show_image

Show image for card previews.

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.card]
show_image = true
```

### include

Include card previews from channels & queries.
If you pass `["#halloy"]`, the channel `#halloy` will show image previews. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels & queries with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.card]
include = []
```

### exclude

Exclude card previews from channels & queries.
If you pass `["#halloy"]`, the channel `#halloy` will not show image previews. You can also exclude all channels & queries by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.card]
exclude = []
```
