# Private Messages

Configure how private messages are displayed in buffers.

- [Private Messages](#private-messages)
  - [Configuration](#configuration)
    - [reroute](#reroute)

## Configuration

### reroute

Reroute private `PRIVMSG` / `NOTICE` traffic from specific users into another
buffer instead of a query buffer.

This only changes where Halloy displays the messages. The messages are still
private and are not visible to other users in the channel.

```toml
# Type: array
# Default: []

[buffer.private_messages]
reroute = [
  { user = "Q", target = { channel = "#noc" } },
  { user = "ChanServ", target = { server = "libera" } },
]
```

Each entry supports:

- `user` - the private-message sender/target to match
- `target` - destination buffer for matching private messages:
  - `{ channel = "#name" }` routes to a channel buffer
  - `{ server = "name" }` routes to the server buffer for the named server
