# Buffer

Buffer settings for Halloy.

## line_spacing

Setting to control spacing between messages in buffers

```toml
# Type: integer
# Values: positive integers
# Default: 0

[buffer]
line_spacing = 4
```

## scroll_position_on_open

Scroll position of the buffer when it opens.

```toml
# Type: string
# Values: "oldest-unread", "newest"
# Default: "oldest-unread"

[buffer]
scroll_position_on_open = "newest"
```
