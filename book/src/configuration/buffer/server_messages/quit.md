# `[buffer.server_messages.quit]`

Server message is sent when a user closes the connection to a channel or server.

**Example**

```toml
[buffer.server_messages.quit]
exclude = ["*"]
include = ["#halloy"]
```

## `enabled`

Control if internal message type is enabled.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `smart`

Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.

- **type**: integer
- **values**: any positive integer
- **default**: not set

## `username_format`

Adjust the amount of information displayed for a username in server messages. If you choose `"short"`, only the nickname will be shown. If you choose `"full"`, the nickname, username, and hostname (if available) will be displayed.

- **type**: string
- **values**: `"full"`, `"short"`
- **default**: `"full"`

## `exclude`

Exclude channels from receiving the server messag.
If you pass `["#halloy"]`, the channel `#halloy` will not receive the server message. You can also exclude all channels by using a wildcard: `["*"]`.

- **type**: array of strings
- **values**: array of any strings
- **default**: `[]`

## `include`

Include channels to receive the server message.
If you pass `["#halloy"]`, the channel `#halloy` will receive the server message. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

- **type**: array of strings
- **values**: array of any strings
- **default**: `[]`