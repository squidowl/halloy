# Reroute

Configure message rerouting for a specific server.

- [Reroute](#reroute)
  - [private_messages](#private_messages)

## private_messages

Reroute private `PRIVMSG` / `NOTICE` traffic from specific users into another
buffer instead of a query buffer.

This only changes where Halloy displays the messages. The messages are still
private and are not visible to other users in the channel.

```toml
# Type: array
# Default: []

[servers.<name>.reroute]
private_messages = [
  { user = "Q", target = { channel = "#foo" } },
  { user = "ChanServ", target = { server = "libera" } },
]
```

Reroutes are scoped to the server section they are configured in.

Each entry supports:

- `user` - the private-message sender/target to match
- `target` - destination buffer for matching private messages:
  - `{ channel = "#name" }` routes to a channel buffer
  - `{ server = "name" }` routes to the server buffer for the named server
