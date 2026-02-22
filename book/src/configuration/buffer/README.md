# Buffer

Buffer settings for Halloy.

- [Buffer](#buffer)
  - [Configuration](#configuration)
    - [line\_spacing](#line_spacing)
    - [scroll\_position\_on\_open](#scroll_position_on_open)
  - [Channel](#channel)
  - [Chat History](#chat-history)
  - [Commands](#commands)
  - [Backlog Separator](#backlog-separator)
  - [Date Separators](#date-separators)
  - [Emojis](#emojis)
  - [Internal Messages](#internal-messages)
  - [Mark as Read](#mark-as-read)
  - [Nickname](#nickname)
  - [Private Messages](#private-messages)
  - [Server Messages](#server-messages)
  - [Status message prefix](#status-message-prefix)
  - [Text Input](#text-input)
  - [Timestamp](#timestamp)
  - [Url](#url)

## Configuration

### line_spacing

Setting to control spacing between messages in buffers

```toml
# Type: integer
# Values: positive integers
# Default: 0

[buffer]
line_spacing = 4
```

### scroll_position_on_open

Scroll position of the buffer when it opens.

```toml
# Type: string
# Values: "oldest-unread", "newest"
# Default: "oldest-unread"

[buffer]
scroll_position_on_open = "newest"
```

## [Channel](channel/)

Channel specific settings

## [Chat History](chat-history/)

IRCv3 Chat History extension settings

## [Commands](commands/)

Commands settings.

## [Backlog Separator](backlog-separator/)

Customize when the backlog separator is displayed within a buffer

## [Date Separators](date-separators/)

Customize how date separators are displayed within a buffer

## [Emojis](emojis/)

Emojis settings.

## [Internal Messages](internal-messages/)

Internal messages are messages sent from Halloy itself.

## [Mark as Read](mark-as-read/)

When to mark a buffer as read

## [Nickname](nickname/)

Customize how nicknames are displayed within a buffer.

## [Private Messages](private-messages/)

Configure private-message-specific buffer behavior.

## [Server Messages](server-messages/)

Server messages are messages sent from an IRC server.

## [Status message prefix](status-message-prefix/)

Status message prefix settings.

## [Text Input](text-input/)

Customize the text input for in buffers.

## [Timestamp](timestamp/)

Customize how timestamps are displayed within a buffer.

## [Url](url/)

Customize how urls behave in buffers
