# Server Messages

Server messages are messages sent from an IRC server.

- [Server Messages](#server-messages)
  - [Example](#example)
  - [Types](#types)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [smart](#smart)
    - [exclude](#exclude)
    - [include](#include)
    - [dimmed](#dimmed)
    - [username\_format](#username_format)
  - [Sub-sections](#sub-sections)
    - [Condense](#condense)

## Example

```toml
# Hide all join messages except for #halloy channel:

[buffer.server_messages.join]
exclude = ["*"]
include = ["#halloy"]

# Hide all part messages

[buffer.server_messages.part]
enabled = false
```

## Types

| **Event Type**        | **Description**                                                                                                                |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `change_host`         | Message is sent when a user changes host                                                                                       |
| `change_mode`         | Message is sent when a mode is set                                                                                             |
| `change_nick`         | Message is sent when a user changes nick                                                                                       |
| `change_topic`        | Message is sent when a channel topic is changed                                                                                |
| `join`                | Message is sent when a user joins a channel                                                                                    |
| `kick`                | Message is sent when a user is kicked from a channel                                                                           |
| `monitored_offline`   | Message is sent when a monitored user goes offline                                                                             |
| `monitored_online`    | Message is sent when a monitored user goes online                                                                              |
| `part`                | Message is sent when a user leaves a channel                                                                                   |
| `quit`                | Message is sent when a user closes the connection to a channel or server                                                       |
| `standard_reply_fail` | Message is sent when a command/function fails or an error with the session                                                     |
| `standard_reply_note` | Message is sent when there is information about a command/function or session                                                  |
| `standard_reply_warn` | Message is sent when there is feedback about a command/function or session                                                     |
| `topic`               | Message is sent when the client joins a channel to inform them of the topic (does not include message sent when topic changes) |

## Configuration

### enabled

Control if internal message type is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.server_messages.<server_message>]
enabled = true
```

### smart

Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.server_messages.<server_message>]
smart = 180
```

### exclude

Exclude channels from receiving the server message.
If you pass `["#halloy"]`, the channel `#halloy` will not receive the server message. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[buffer.server_messages.<server_message>]
exclude = ["*"]
```

### include

Include channels to receive the server message.
If you pass `["#halloy"]`, the channel `#halloy` will receive the server message. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[buffer.server_messages.<server_message>]
include = ["#halloy"]
```

### dimmed

Dim condensed server message.  Either automatically, based on text/background colors (by setting to `true`), or specify a dimming value in the range `0.0` (transparent) to `1.0` (no dimming).

```toml
# Type: bool or float
# Values: true, false, or float
# Default: true

[buffer.server_messages.<server_message>]
dimmed = true
```

### username_format

Adjust the amount of information displayed for a username in server messages. If you choose `"short"`, only the nickname will be shown. If you choose `"full"`, the nickname, username, and hostname (if available) will be displayed.

> 💡 Not all server messages uses this setting.

```toml
# Type: string
# Values: "full", "short"
# Default: "full"

[buffer.server_messages.<server_message>]
username_format = "full"
```

## Sub-sections

### [Condense](condense.md)

Condense multiple consecutive server messages into a single abbreviated message
