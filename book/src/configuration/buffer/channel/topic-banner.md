# Topic Banner

Topic banner settings within a channel buffer.

- [Topic Banner](#topic-banner)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [max\_lines](#max_lines)

## Configuration

### enabled

Control if topic banner should be shown or not by default.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.channel.topic_banner]
enabled = true
```

### max_lines

Amount of visible lines before you have to scroll in topic banner.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 2

[buffer.channel.topic_banner]
max_lines = 2
```
