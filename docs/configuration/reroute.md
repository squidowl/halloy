# Reroute

Configure message rerouting for a specific server.

- [Reroute](#reroute)
  - [query](#query)
  - [notice](#notice)


Reroutes private `PRIVMSG` / `NOTICE` traffic from users into another
buffer instead of a query buffer.

Each entry supports:

- `user` - the private-message sender/target to match, "\*" will specify all users
- `target` - destination buffer for matching private messages:
  - `{ channel = "#name" }` routes to a channel buffer
  - `"server"` routes to the server buffer

## query

Reroute private `PRIVMSG` traffic from specific users into another
buffer instead of a query buffer.

This only changes where Halloy displays the messages. The messages are still
private and are not visible to other users in the channel.

```toml
# Type: array
# Default: []

[servers.<name>.reroute]
query = [
  { user = "Q", target = { channel = "#foo" } },
  { user = "ChanServ", target = "server" },
]
```

## notice

Reroute private `NOTICE` traffic from specific users into another
buffer instead of a query buffer.

This only changes where Halloy displays the notices. The notices are still
private and are not visible to other users in the channel.

```toml
# Type: array
# Default: [{ user = "*", target = "server" }]

[servers.<name>.reroute]
notice = [
  { user = "MyBot", target = { channel = "#my-channel" } },
  { user = "*", target = "server" },
]
```
