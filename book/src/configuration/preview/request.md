# Request

Request settings for previews.

- [Request](#request)
  - [Configuration](#configuration)
    - [user\_agent](#user_agent)
    - [timeout\_ms](#timeout_ms)
    - [max\_image\_size](#max_image_size)
    - [max\_scrape\_size](#max_scrape_size)
    - [concurrency](#concurrency)
    - [delay\_ms](#delay_ms)
    - [image\_cache\_max\_size](#image_cache_max_size)
    - [image\_cache\_trim\_interval](#image_cache_trim_interval)

## Configuration

### user_agent

Some servers will only send opengraph metadata to browser-like user agents. We default to `WhatsApp/2` for wide compatibility.

```toml
# Type: string
# Values: any string
# Default: "WhatsApp/2"

[preview.request]
user_agent = "WhatsApp/2"
```

### timeout_ms

Request timeout in milliseconds. Defaults is 10s.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 10000

[preview.request]
timeout_ms = 10000
```
 
### max_image_size

Max image size in bytes. This prevents downloading responses that are too big. Default is 10mb.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 10485760

[preview.request]
max_image_size = 10485760
```

### max_scrape_size

Max bytes streamed when scraping for opengraph metadata before cancelling the request. This prevents downloading responses that are too big. Default is 500kb.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 512000

[preview.request]
max_scrape_size = 512000
```

### concurrency

Number of allowed concurrent requests for fetching previews. Reduce this to prevent rate-limiting.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 4

[preview.request]
concurrency = 4
```

### delay_ms

Number of milliseconds to wait before requesting another preview when number of requested previews > `concurrency`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 500

[preview.request]
delay_ms = 500
```

### image_cache_max_size

Maximum size in MB for cached preview images.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 500

[preview.request]
image_cache_max_size = 500
```

### image_cache_trim_interval

Run image cache trimming every N successful image saves. Set to `0` to disable periodic trimming (a first-save trim still runs once per app session).

```toml
# Type: integer
# Values: any non-negative integer
# Default: 32

[preview.request]
image_cache_trim_interval = 32
```
