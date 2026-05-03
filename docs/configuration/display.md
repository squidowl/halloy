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

## `nickname`

Metadata to include when rendering user nicknames in message buffers.

```toml
# Type: array of strings
# Values: "display-name", "pronouns", "color"
# Default: ["display-name"]

[display]
nickname = ["display-name"]
```

Examples:

```toml
[display]
nickname = ["display-name", "pronouns"]
```

This renders as:
- `["display-name"]` -> `Casper (casperstorm)` when display name (`Casper`) is set, otherwise `casperstorm`
- `["pronouns"]` -> `casperstorm (he/him)` when pronouns are set, otherwise `casperstorm`
- `["display-name", "pronouns"]` -> `Casper (casperstorm, he/him)`, omitting missing metadata
- `["display-name"]` -> `Casper (casperstorm)` when display name (`Casper`) is set, with text color as specified by the user's metadata

## `nicklist_nickname`

Metadata to include when rendering user nicknames in the nicklist.

```toml
# Type: array of strings
# Values: "display-name", "pronouns", "color"
# Default: ["display-name"]

[display]
nicklist_nickname = ["display-name"]
```
