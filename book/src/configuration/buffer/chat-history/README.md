# Chat History

IRCv3 [`chathistory`](https://ircv3.net/specs/extensions/chathistory) extension settings

- [Chat History](#chat-history)
  - [Configuration](#configuration)
    - [infinite\_scroll](#infinite_scroll)

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
