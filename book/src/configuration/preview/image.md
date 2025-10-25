# Image

Specific image preview settings.

- [Image](#image)
  - [Example](#example)
  - [Configuration](#configuration)
    - [action](#action)
    - [include](#include)
    - [exclude](#exclude)

## Example 

```toml
[preview.image]
exclude = ["*"] # hide image previews in all channels
include = ["#halloy"] # show image previews in #halloy
```

## Configuration

### action

Action when clicking on a image. `open-url` will open the image in the browser, and `preview` will display a larger version of the image in-app.

```toml
# Type: string
# Values: "open-url", "preview"
# Default: "preview"

[preview.image]
action = "preview"
```

### include

Include image previews from channels & queries.
If you pass `["#halloy"]`, the channel `#halloy` will show image previews. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels & queries with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.image]
include = []
```

### exclude

Exclude image previews from channels & queries.
If you pass `["#halloy"]`, the channel `#halloy` will not show image previews. You can also exclude all channels & queries by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.image]
exclude = []
```
