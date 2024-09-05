# `[buffer.channel.topic]`

The `[buffer.channel.topic]` section allows you to customize the topic banner within a channel buffer.

**Example**

```toml
[buffer.channel.topic]
enabled = true
max_lines = 2
```

## `enabled`

Control if topic should be shown or not by default.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `false`

## `max_lines`

Amount of visible lines before you have to scroll in topic banner.

- **type**: integer
- **values**: any positive integer
- **default**: `2`
