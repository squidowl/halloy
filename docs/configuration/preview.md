# Preview

URL preview settings for Halloy.

## `enabled`

Enable or disable previews globally with a boolean, or selectively enable them for URLs matching specific regex patterns.

```toml
# Type: boolean or array of strings
# Values: true, false, or array of regex patterns
# Default: true

[preview]
enabled = true
```

Only show previews for matching URLs:

> 💡 Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This allows you to write the regex without escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.
>
> Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`

```toml
[preview]
enabled = [
    '''https?://(www\.)?imgur\.com/.*''', 
    '''https?://(www\.)?dr\.dk/.*'''
]
```

## `exclude`

Exclude URLs from showing previews by providing regex patterns.

```toml
# Type: array of strings
# Values: array of regex patterns
# Default: []

[preview]
exclude = []
```

Prevent previews from showing for matching URLs:

> 💡 Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This allows you to write the regex without escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.
>
> Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`

```toml
[preview]
exclude = [
    '''https?://(www\.)?example\.com/.*''', 
    '''https?://(www\.)?spam-site\.net/.*'''
]
```

## `max_per_message`

Maximum number of previews to show for a single message.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 1

[preview]
max_per_message = 1
```

## `card`

Specific card preview settings.

```toml
[preview.card]
exclude = "*" # hide card previews in all channels
include = { channels = ["#halloy"] } # show card previews in #halloy
```

### `show_image`

Show image for card previews.

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.card]
show_image = true
```

### `round_image_corners`

Round the corners of the image in the card preview (if shown).

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.card]
round_image_corners = true
```

### `max_width`

Maximum width of the card in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 400.0

[preview.card]
max_width = 400.0
```

### `description_max_height`

Maximum height of the description text in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 100.0

[preview.card]
description_max_height = 100.0
```

### `image_max_height`

Maximum height of the image in the card preview in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 200.0

[preview.card]
image_max_height = 200.0
```

### `exclude`

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

### `include`

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

## `image`

Specific image preview settings.

```toml
[preview.image]
exclude = "*" # hide image previews in all channels
include = { channels = ["#halloy"] } # show image previews in #halloy
```

### `action`

Action when clicking on a image. `open-url` will open the image in the browser, and `preview` will display a larger version of the image in-app.

```toml
# Type: string
# Values: "open-url", "preview"
# Default: "preview"

[preview.image]
action = "preview"
```

### `round_corners`

Round the corners of the image.

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.image]
round_corners = true
```

### `max_width`

Maximum width of the image in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 550.0

[preview.image]
max_width = 550.0
```

### `max_height`

Maximum height of the image in pixels.

```toml
# Type: number
# Values: any positive number
# Default: 350.0

[preview.image]
max_height = 350.0
```

### `exclude`

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

### `include`

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

## `image_cache`

Settings to control how the image cache is managed.  The cache is stored in:

* Windows: `%AppData%\Roaming\Local\halloy\previews\images\`
* Mac: `~/Library/Caches/halloy/previews/images/` or `$HOME/.cache/halloy/previews/images/`
* Linux: `$XDG_CACHE_HOME/halloy/previews/images/`, `$HOME/.cache/halloy/previews/images/`, or `$HOME/.var/app/org.squidowl.halloy/cache/halloy/previews/images/` (Flatpak)

### `max_size`

Maximum size in MB for cached preview images, or `"unlimited"` for an uncapped image cache (not recommended).

```toml
# Type: integer
# Values: any non-negative integer or "unlimited"
# Default: 500

[preview.request.image_cache]
max_size = 500
```

### `trim_interval`

Run image cache trimming every N successful image saves. Set to `"first-save-only"` to disable periodic trimming, and only trim on the first save to the image cache per app session.

```toml
# Type: integer
# Values: any non-negative integer or "first-save-only"
# Default: 32

[preview.request.image_cache]
trim_interval = 32
```

## `request`

Request settings for previews.

### `user_agent`

Some servers will only send opengraph metadata to browser-like user agents. We default to `WhatsApp/2` for wide compatibility.

```toml
# Type: string
# Values: any string
# Default: "WhatsApp/2"

[preview.request]
user_agent = "WhatsApp/2"
```

### `timeout_ms`

Request timeout in milliseconds. Defaults is 10s.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 10000

[preview.request]
timeout_ms = 10000
```
 
### `max_image_size`

Max image size in bytes. This prevents downloading responses that are too big. Default is 10mb.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 10485760

[preview.request]
max_image_size = 10485760
```

### `max_scrape_size`

Max bytes streamed when scraping for opengraph metadata before cancelling the request. This prevents downloading responses that are too big. Default is 500kb.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 512000

[preview.request]
max_scrape_size = 512000
```

### `concurrency`

Number of allowed concurrent requests for fetching previews. Reduce this to prevent rate-limiting.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 4

[preview.request]
concurrency = 4
```

### `delay_ms`

Number of milliseconds to wait before requesting another preview when number of requested previews > `concurrency`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 500

[preview.request]
delay_ms = 500
```
