# Image

Specific image preview settings.

- [Image](#image)
  - [Example](#example)
  - [Configuration](#configuration)
    - [action](#action)
    - [round\_corners](#round_corners)
    - [max\_width](#max_width)
    - [max\_height](#max_height)
    - [include](#include)
    - [exclude](#exclude)

## Example 

```toml
[preview.image]
exclude = "*" # hide image previews in all channels
include = { channels = ["#halloy"] } # show image previews in #halloy
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

### round_corners

Round the corners of the image.

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.image]
round_corners = true
```

### max_width

Maximum width of the image in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 550.0

[preview.image]
max_width = 550.0
```

### max_height

Maximum height of the image in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 350.0

[preview.image]
max_height = 350.0
```

### exclude

[Exclusion conditions](/configuration/conditions.md) for when image previews
will be hidden. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[preview.image]
exclude = { criteria = [{ server_message = "topic", channel = "#linux" }] } # exclude previews from topic messages in #linux
```

### include

[Inclusion conditions](/configuration/conditions.md) for when image previews
will be shown. Image previews will be shown for all conditions (when enabled)
unless explicitly excluded, so this setting is only relevant when combined with
the `exclude` setting.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[preview.image]
include = { users = ["BridgeBot"] }
```
