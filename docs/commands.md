# Commands

Commands in Halloy are prefixed with `/`.

- [Commands](#commands)
  - [Example](#example)
  - [Types](#types)

## Example

```sh
/me says halloy!
```

## Types

Halloy will first try to run below commands, and lastly send it directly to the server.
The argument(s) for a command are shown in [tooltips](configuration/tooltips.md), and those marked with a `*` will show an additional tooltip with argument-specific information on mouseover.

You can configure custom command aliases in [`buffer.commands.aliases`](configuration/buffer#aliases).
Aliases are resolved before built-in commands and take precedence when they use the same name.

| Command         | Alias        | Description                                                                              |
| --------------- | ------------ | ---------------------------------------------------------------------------------------- |
| `away`          |              | Mark yourself as away. If already away, the status is removed                            |
| `chathistory`   |              | Retrieve message history[^5]                                                             |
| `clear`         |              | Clear the message history in the current buffer                                          |
| `cleartopic`    | `ct`         | Clear the topic of a channel[^1]                                                         |
| `connect`       |              | Connect to a server, specified by URL or current server if disconnected[^8]              |
| `ctcp`          |              | Client-To-Client requests[^2]                                                            |
| `delay`         |              | Delay the specified number of seconds[^7]                                                |
| `detach`        |              | Hide the channel, but leave the bouncer's connection to the channel active[^5][^6]       |
| `exec`          |              | Run a local shell command and send the first line of stdout to the current buffer[^11]   |
| `format`        | `f`          | Format text with markdown and colors                                                     |
| `format-me`     |              | Send an action with markdown and colors                                                  |
| `format-msg`    |              | Open a pane with a target and send an optional message with markdown and colors          |
| `format-notice` |              | Send a notice message to a target with markdown and colors                               |
| `hop`           | `rejoin`     | Part the current channel and join a new one                                              |
| `join`          | `j`          | Join channel(s) with optional key(s)[^9][^10]                                            |
| `kick`          |              | Kick a user from a channel[^1]                                                           |
| `knock`         |              | Request an invite from an invitation-only channel[^5]                                    |
| `list`          |              | List channel(s) on the server[^5]                                                        |
| `me`            | `describe`   | Send an action message to the channel                                                    |
| `mode`          | `m`          | Set mode(s) on a channel or retrieve the current mode(s) set[^3]                         |
| `monitor`       |              | System to notify when users become online/offline[^5]                                    |
| `motd`          |              | Request the message of the day                                                           |
| `msg`           | `query`      | Open a pane with a target and send an optional message                                   |
| `nick`          |              | Change your nickname on the current server                                               |
| `notice`        |              | Send a notice message to a target                                                        |
| `part`          | `leave`      | Leave and close channel(s)/quer(ies) with an optional reason [^4]                        |
| `plain`         | `p`          | Send text with markdown and colors disabled                                              |
| `plain-me`      |              | Send an action with markdown and colors disabled                                         |
| `plain-msg`     |              | Open a pane with a target and send an optional message with markdown and colors disabled |
| `plain-notice`  |              | Send a notice message to a target with markdown and colors disabled                      |
| `quit`          | `disconnect` | Disconnect from the server with an optional reason                                       |
| `raw`           |              | Send data to the server without modifying it                                             |
| `reconnect`     |              | Reconnect to a current server if disconnected                                            |
| `search`        |              | Search message history by content, sender, and/or time[^5]                               |
| `setname`       |              | Change your realname[^5]                                                                 |
| `sysinfo`       |              | Send system information (OS, CPU, memory, GPU, uptime)                                   |
| `topic`         | `t`          | Retrieve the topic of a channel or set a new topic[^1]                                   |
| `whois`         |              | Retrieve information about user(s)                                                       |

[^1]: The `channel` argument can be skipped when used in a channel buffer to target the channel in the buffer.
[^2]: The `nick` argument can be skipped when used in a query buffer to target the other user in the buffer.
[^3]: The `target` argument can be skipped; in a channel buffer it will target the channel in the buffer, in a query buffer it will target the other user in the buffer, and in a server buffer it will target your user.
[^4]: The `targets` argument can be skipped; in a channel or query buffer it will target the current buffer.
[^5]: Command must be supported by the bouncer/server to be executed successfully; if not supported then the command will not appear in the command picker.
[^6]: See [soju](https://soju.im/)'s [documentation on detaching from channels](https://man.sr.ht/chat.sr.ht/bouncer-usage.md#detaching-from-channels) for more information.
[^7]: Can only be used in [on_connect](./configuration/servers#on_connect).
[^8]: Connections made will not be remembered after quitting Halloy (i.e. when next starting Halloy it will not re-make the connection).  Add the connection information to the [servers](configuration/servers) section in the configuration file.
[^9]: Channels joined will not be remembered after quitting Halloy (i.e. when next starting Halloy it will not re-join the channels).  Add the channel information to the [channels setting for the server](configuration/servers#channels) in the configuration file.
[^10]: If not joined to the channel in the buffer, then the `chanlist` argument can be skipped to target the channel in the buffer.
[^11]: The command is executed locally with `sh -c` on Unix-like systems and `cmd /C` on Windows. Only the first non-empty line of stdout is used. If that line starts with `/`, it is treated as a command; otherwise it is sent as a normal message. `/exec` is disabled by default and must be explicitly enabled in [`buffer.commands.exec`](configuration/buffer#exec).
