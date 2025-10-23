# Reduce Noise

It's not uncommon for channels to have many server messages for every regular message, resulting in a low signal to noise ratio.  Halloy has various settings that can help reduce the number of visible server messages in the chat log.  This guide will cover some of those settings.

## Disable Topic Messages

Most servers and bouncers will send a message with the topic every time Halloy joins a channel.  Since topics rarely change, it's often useful to hide these messages altogether with disabling the topic [server message setting](../configuration/buffer/server-messages):

```toml
[buffer.server_messages.topic]
enabled = false
```

Note, this will not hide the messages sent when a topic changes, only the topic messages sent on first connection to a channel.

If topic messages are hidden but a reminder of the current topic is still desired, then a topic banner can be enabled to appear at the top of each pane.  Either with the label icon button in the pane's title bar, or with the [topic banner settings](../configuration/buffer/channel/topic-banner.md):

```toml
[buffer.channel.topic_banner]
enabled = true
max_lines = 2
```

## Smart Filters

[Smart filters](../configuration/buffer/server-messages/README.md#smart) can be used for server messages to hide messages for users that have not sent a message recently.  For example, to hide part messages for any user that has not sent a message within the 15 minutes prior to their parting:

```toml
[buffer.server_messages.part]
smart = 900
```

For many channels join, part, quit, and nickname changes make up a lot of noise and usually aren't relevant if the user hasn't been active.  To smart filter those messages these settings can be used:

```toml
[buffer.server_messages]
join.smart = 900
part.smart = 900
quit.smart = 900
change_nick.smart = 900
```

Smart filters can also be applied to [internal messages](../configuration/buffer/internal-messages/) as well.  For example, to hide any connect or disconnect message older than five minutes, use these settings:

```toml
[buffer.internal_messages]
success.smart = 300
error.smart = 300
```

## Condense Server Messages

It may be preferable to not hide any server messages, in which case an alternative to filtering is to [condense server messages](../configuration/buffer/server-messages/condense.md).  This setting will combine multiple server messages into a one server message with a shortened style.  To enable condensed messages these settings can be used:

```toml
[buffer.server_messages.condense]
messages = ["join", "part", "quit"]
dimmed = true
```

When using condensed messages, it is recommended that you specify colors for the condensed messages in your [theme](../configuration/themes/)).  Those colors will be used for the abbreviations used in the condensed messages.  For example, these theme settings could be added:

```toml
[buffer.server_messages]
join = "#efff95"
part = "#ff6b77"
quit = "#ff6b77"
```

## Ignore

If dealing with a noisy user or bot, an [ignore filter](../configuration/servers/filters.md#ignore) can be used to hide their messages.  For example, to hide messages produced by `ChanServ` in the `#halloy` channel, the following setting can be used:

```toml
[servers.libera.filters]
ignore = ["#halloy ChanServ"]
```
