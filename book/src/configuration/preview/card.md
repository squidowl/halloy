# Card

Specific card preview settings.

- [Card](#card)
  - [Example](#example)
  - [Configuration](#configuration)
    - [show\_image](#show_image)
    - [round\_image\_corners](#round_image_corners)
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
