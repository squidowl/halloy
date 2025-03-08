# `[highlights]`

Application wide highlights.

**Example**

```toml
# Enable nickname highlights in channel #halloy.
[highlights.nickname]
exclude = ["*"]
include = ["#halloy"]

# Highlight on 'boat' and 'car' in any channel.
[[highlights.match]]
words = ["boat", "car"]
case_insensitive = true

# Highlight when regex matches in any channel.
[[highlights.match]]
regex = "(?i)\\bcasper\\b"
```

## `[highlights.nickname]`

Nickname highlights.

### `exclude`

Channels in which you won’t be highlighted.
If you pass `["#halloy"]`, you won’t be highlighted by nickname in that channel. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[highlights.nickname]
exclude = ["*"]
```

### `include`

Channels in which you will be highlighted.
If you pass `["#halloy"]`, you will be highlighted by nickname in that channel. You can also include all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: ["*"]

[highlights.nickname]
include = ["*"]
```

## `[[highlights.match]]`

Highlight based on matches.

### `words`

You can set words to be highlighted when they are written.

Example shows word matches, which will trigger on `"word1"`, `"word2"` or `"word3"` in any channel.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[[highlights.match]]
words = ["word1", "word2", "word3"]
```

### `case_insensitive`

This option is only available when using `words` as the match type.
You can choose whether or not to trigger regardless of case.

```toml
# Type: boolean
# Values: true, false
# Default: false

[[highlights.match]]
# words = ["word1", "word2", "word3"] - requires words
case_insensitive = true
```

### `regex`

Match based on regex.

Example shows a regex that matches the word "casper", regardless of case  and only when it appears as a whole word in any channel.

```toml
# Type: string
# Values: any string
# Default: not set

[[highlights.match]]
regex = "(?i)\bcasper\b"
```

### `exclude`

Channels in which you won’t be highlighted.
If you pass `["#halloy"]`, you won’t be highlighted by nickname in that channel. You can also exclude all channels by using a wildcard: `["*"]`.

Example shows a regex match which will be excluded in from `#noisy-channel`

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[[highlights.match]]
# regex = "(?i)\bcasper\b"
exclude = ["#noisy-channel"]
```

### `include`

Channels in which you will be highlighted.
If you pass `["#halloy"]`, you will be highlighted by nickname in that channel. You can also include all channels by using a wildcard: `["*"]`.

Example shows a regex match which will only try to match in `#halloy` channel.

```toml
# Type: array of strings
# Values: array of any strings
# Default: ["*"]

[[highlights.match]]
# regex = "(?i)\bcasper\b"
# exclude = ["*"]
include = ["#halloy"]
```