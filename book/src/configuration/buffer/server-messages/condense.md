# Condense

Condense multiple consecutive server messages into a single abbreviated message.

- [Condense](#condense)
  - [Configuration](#configuration)
    - [messages](#messages)
    - [dimmed](#dimmed)
    - [format](#format)
    - [icon](#icon)

## Configuration

### messages

 Message type(s) to condense. Supported types:

| **Event Type** | **Symbol** |
| -------------- | ---------- |
| `change-host`  | `→`        |
| `change-nick`  | `→`        |
| `join`         | `+`        |
| `part`         | `-`        |
| `quit`         | `-`        |
| `kick`         | `!`        |

The color and font style of the symbols is taken from the theme setting for that event type.

```toml
# Type: array of strings
# Values: ["change-host", "change-nick", "join", "kick", "part", "quit"]
# Default: []

[buffer.server_messages.condense]
messages = ["change-nick", "join", "part", "quit"]
```

### dimmed

Dim condensed messages.  Either automatically, based on text/background colors (by setting to `true`), or specify a dimming value in the range `0.0` (transparent) to `1.0` (no dimming).

```toml
# Type: bool or float
# Values: true, false, or float
# Default: true

[buffer.server_messages.condense]
dimmed = true
```

### format

How to format condensed messages:

- `"brief"`:  Only show changes to channel state.  If a user joins then leaves, then do not show any message.  If a user joins, leaves, then joins again, then show that they joined the channel (`+`).
- `"detailed"`: Include messages that do not change channel state, but do not show repeated events.  If a user joins then leaves, show a condensed message with both events (`+-`).  But, if a user joins and leaves many times in a row, only indicate that they left and re-joined (i.e. still `+-`).
- `"full`":  Include all messages in the condensed message.  If a user joins and leaves three times, then show a symbol for each event (`+-+-+-`).

```toml
# Type: string
# Values: "brief", "detailed", "full"
# Default: "brief"

[buffer.server_messages.condense]
format = "full"
```

### icon

Marker style for condensed server messages.

```toml
# Type: string
# Values: "none", "chevron", "dot"
# Default: "none"

[buffer.server_messages.condense]
icon = "chevron"
```
