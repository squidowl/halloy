# Chat History

IRCv3 [`chathistory`](https://ircv3.net/specs/extensions/chathistory) extension settings

- [Chat History](#chat-history)
  - [Configuration](#configuration)
    - [infinite\_scroll](#infinite_scroll)
    - [max\_messages](#max_messages)

## Configuration

### infinite_scroll

Automatically request older history when scrolling to the top of a channel/query buffer

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.chathistory]
infinite_scroll = true
```

### max_messages

Set the number of messages recorded to the history files on disk

> 💡 Setting this to 0 (zero) will disable all history from being written to disk. This will also result in buffers not populating previous messages in a new session.

```toml
# Type: integer
# Values: any positive integer
# Default: 10000

[buffer.chathistory]
max_messages = 10000
```
