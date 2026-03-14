# Chat History

IRCv3 [`chathistory`](https://ircv3.net/specs/extensions/chathistory) extension settings

- [Chat History](#chat-history)
  - [Configuration](#configuration)
    - [infinite\_scroll](#infinite_scroll)
    - [persist](#persist)

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

### persist

Write chat history to disk. When set to `false`, no new history files will be written.

> ⚠️ Existing history files are still loaded normally and must be removed manually if desired.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.chathistory]
persist = true
```
