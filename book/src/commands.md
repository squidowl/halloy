# Commands

Commands in Halloy are prefixed with `/`.

Example

```
/me says halloy!
```

Halloy will first try to run below commands, and lastly send it directly to the server.

| Command   | Alias      | Description                                                   |
| --------- | ---------- | ------------------------------------------------------------- |
| `away`    |            | Mark yourself as away. If already away, the status is removed |
| `join`    | `j`        | Join channel(s) with optional key(s)                          |
| `me`      | `describe` | Send an action message to the channel                         |
| `mode`    | `m`        | Set mode(s) on a channel or retrieve the current mode(s) set  |
| `monitor` |            | System to notify when users become online/offline             |
| `msg`     |            | Open a query with a nickname and send an optional message     |
| `nick`    |            | Change your nickname on the current server                    |
| `part`    | `leave`    | Leave channel(s) with an optional reason                      |
| `quit`    |            | Disconnect from the server with an optional reason            |
| `raw`     |            | Send data to the server without modifying it                  |
| `topic`   | `t`        | Retrieve the topic of a channel or set a new topic            |
| `whois`   |            | Retrieve information about user(s)                            |
| `ctcp`    |            | Client-To-Client requests                                     |
