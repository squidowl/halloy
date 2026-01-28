# Card

Specific card preview settings.

- [Card](#card)
  - [Example](#example)
  - [Configuration](#configuration)
    - [show\_image](#show_image)
    - [round\_image\_corners](#round_image_corners)
    - [max\_width](#max_width)
    - [description\_max\_height](#description_max_height)
    - [image\_max\_height](#image_max_height)
    - [include](#include)
    - [exclude](#exclude)

## Example 

```toml
[preview.card]
exclude = "*" # hide card previews in all channels
include = { channels = ["#halloy"] } # show card previews in #halloy
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

### round_image_corners

Round the corners of the image in the card preview (if shown).

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.card]
round_image_corners = true
```

### max_width

Maximum width of the card in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 400.0

[preview.card]
max_width = 400.0
```

### description_max_height

Maximum height of the description text in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 100.0

[preview.card]
description_max_height = 100.0
```

### image_max_height

Maximum height of the image in the card preview in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 200.0

[preview.card]
image_max_height = 200.0
```

### exclude

[Exclusion conditions](/configuration/conditions.md) for when card previews will
be hidden. Inclusion conditions will take precedence over exclusion conditions.
You can also exclude all conditions by setting to `"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[preview.card]
exclude = { criteria = [{ server_message = "topic", channel = "#linux" }] } # exclude previews from topic messages in #linux
```

### include

[Inclusion conditions](/configuration/conditions.md) for when card previews will
be shown. Card previews will be shown for all conditions (when enabled) unless
explicitly excluded, so this setting is only relevant when combined with the
`exclude` setting.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[preview.card]
include = { users = ["BridgeBot"] }
```
