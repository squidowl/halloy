# Private Messages

Configure how private messages are displayed in buffers.

- [Private Messages](#private-messages)
  - [Configuration](#configuration)
    - [reroute](#reroute)

## Configuration

### reroute

Reroute private `PRIVMSG` / `NOTICE` traffic from specific users into a channel
buffer instead of a query buffer.

This only changes where Halloy displays the messages. The messages are still
private and are not visible to other users in the channel.

```toml
# Type: array
# Default: []

[buffer.private_messages]
reroute = [
  { user = "ChanServ", channel = "#halloy" },
]
```

Each entry currently supports:

- `user` - the private-message sender/target to match
- `channel` - the channel buffer where matching private messages should appear
