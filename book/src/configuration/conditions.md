# Inclusion/Exclusion Conditions

Inclusion/exclusion conditions can be specified by arrays of users, channels,
servers, and/or combined criteria.  A combined criteria can include a user,
channel, and/or server, but will only be matched if **all** are matched.  The
conditions are met if **any** individual condition is matched.  Inclusion
conditions will take precedence over exclusion conditions.  You can specify all
conditions by setting to `"all"` or `"*"`.

## Examples

This example excludes all messages in `#halloy` except for messages from `GH-Bot`.

```toml
include = { users = ["#GH-Bot"] }
exclude = { channels = ["#halloy"] }
```

This example excludes messages from `GH-Bot` and `ChanServ` in `#halloy`.

```toml
exclude = { criteria = [{ channel = "#halloy", user = "GH-Bot" }, { channel = "#halloy", user = "ChanServ" }] }
```

Using regular TOML table can help with legibility, as in this example excluding
notifications for highlights by `GH-Bot` and `ChanServ` in `#halloy`.

```toml
[notifications.highlight.exclude]
criteria = [{ channel = "#halloy", user = "GH-Bot" }, 
            { channel = "#halloy", user = "ChanServ" }]
```

This example excludes messages in `#halloy` on the `libera` server only.

```toml
exclude = { criteria = [{ channel = "#halloy", server = "libera" }] }
```

This example excludes messages in all servers except `libera`.

```toml
include = { servers = "libera" }
exclude = "all"
```
