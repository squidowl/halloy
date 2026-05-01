# Display

Display settings for Halloy.

## `direction_arrows`

Customize the arrows used for directional messages such as join, part, quit, kick, and CTCP messages.

### `left`

Arrow shown for left-facing events.

```toml
# Type: string
# Values: any string
# Default: "←"

[display]
direction_arrows = { left = "<" }
```

### `right`

Arrow shown for right-facing events.

```toml
# Type: string
# Values: any string
# Default: "→"

[display]
direction_arrows = { right = ">" }
```

## `truncation_character`

Customize the character used to indicate a nickname was truncated.

```toml
# Type: character
# Values: any character
# Default: "…"

[display]
truncation_character = '-'
```

## `decode_urls`

Whether to automatically decode urls in messages, otherwise the URLs
will appear exactly as sent.  E.g. when enabled `https://bücher.de` will appear as `https://bücher.de`
`https://ja.wikipedia.org/wiki/%E9%87%8D%E9%9F%B3%E3%83%86%E3%83%88`
will be displayed as `https://ja.wikipedia.org/wiki/重音テト`.

```toml
# Type: boolean
# Values: true, false
# Default: true

[display]
decode_urls = false
```
