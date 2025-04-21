# `[preview]`

URL preview settings for Halloy.

## `enabled`

Enable or disable previews globally

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview]
enabled = true
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
# Values: any positive integer
# Default: 10000

[preview.request]
timeout_ms = 10000
```
 
### `max_image_size`

Max image size in bytes. This prevents downloading responses that are too big. Default is 10mb.

```toml
# Type: integer
# Values: any positive integer
# Default: 10485760

[preview.request]
max_image_size = 10485760
```

### `max_scrape_size`

Max bytes streamed when scraping for opengraph metadata before cancelling the request. This prevents downloading responses that are too big. Default is 500kb.

```toml
# Type: integer
# Values: any positive integer
# Default: 512000

[preview.request]
max_scrape_size = 512000
```

### `concurrency`

Number of allowed concurrent requests for fetching previews. Reduce this to prevent rate-limiting.

```toml
# Type: integer
# Values: any positive integer
# Default: 4

[preview.request]
concurrency = 4
```

### `delay_ms`

Number of milliseconds to wait before requesting another preview when number of requested previews > `concurrency`.

```toml
# Type: integer
# Values: any positive integer
# Default: 500

[preview.request]
delay_ms = 500
```


## `image`

Specific image preview settings.

### `action`

Action when clicking on a image. `open-url` will open the image in the browser, and `preview` will display a larger version of the image in-app.

```toml
# Type: string
# Values: "open-url", "preview"
# Default: "preview"

[preview.image]
action = "preview"
```


### `include`

Include image previews from channels.
If you pass `["#halloy"]`, the channel `#halloy` will show image previews. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.image]
include = []
```

### `exclude`

Exclude image previews from channels.
If you pass `["#halloy"]`, the channel `#halloy` will not show image previews. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.image]
exclude = []
```

### Example 

```toml
[preview.image]
exclude = ["*"] # hide image previews in all channels
include = ["#halloy"] # show image previews in #halloy
```

## `card`

Specific card preview settings.

### `show_image`

Show image for card previews.

```toml
# Type: boolean
# Values: true, false
# Default: true

[preview.card]
show_image = true
```

### `include`

Include card previews from channels.
If you pass `["#halloy"]`, the channel `#halloy` will show image previews. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.card]
include = []
```


### `exclude`

Exclude card previews from channels.
If you pass `["#halloy"]`, the channel `#halloy` will not show image previews. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[preview.card]
exclude = []
```
### Example 

```toml
[preview.card]
exclude = ["*"] # hide card previews in all channels
include = ["#halloy"] # show card previews in #halloy
```

