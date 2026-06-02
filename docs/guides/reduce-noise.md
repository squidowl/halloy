# Reduce Noise

It's not uncommon for channels to have many server messages for every regular message, resulting in a low signal to noise ratio.  Halloy has various settings that can help reduce the number of visible server messages in the chat log.  This guide will cover some of those settings.

## Condense Server Messages

Enabled by default, [condensing](../configuration/buffer#condense) server messages keeps them visible while reducing their visual impact.  However, if a channel is very active then lists of condensed messages can still grow long. The [`max`](../configuration/buffer#max) setting will limit how many condensed messages are shown in each block:

```toml
[buffer.server_messages.condense]
max = 15
```

## Smart Filters

Some server message types can't be condensed, in which case [smart filters](../configuration/buffer#smart) can be used to hide messages for users that have not sent a message recently.  For example, to hide part messages for any user that has not sent a message within the 15 minutes prior to their parting:

```toml
[buffer.server_messages.part]
smart = 900 # seconds = 15 minutes
```

For example, many channels join, part, quit, and nickname changes make up a lot of noise and usually aren't relevant if the user hasn't been active.  To smart filter those messages these settings can be used:

```toml
[buffer.server_messages]
join.smart = 900
part.smart = 900
quit.smart = 900
change_nick.smart = 900
```

Automated away messages will often by sent in response to every message received.  A smart filter for away messages will hide new away messages if an away message has been received within the specified time frame.  So, to hide away messages if one has already been received in the last 12 minutes, the following setting can be used:

```toml
[buffer.server_messages.away]
smart = 720 # seconds = 12 minutes
```

Smart filters can also be applied to [internal messages](../configuration/buffer#internal_messages).  These settings can be used to hide old connect & disconnect messages which are no longer pertinent.  For example, to hide any connect or disconnect message older than five minutes, use these settings:

```toml
[buffer.internal_messages]
success.smart = 300 # seconds = 5 minutes
error.smart = 300 # seconds = 5 minutes
```

## Disable

Some server messages may not be of interest, in which case they can be hidden by disabling them;  when server messages are disabled they will be hidden, but they are not discarded and can be revealed again when they are re-enabled.  For example, host change server messages are often not of interest and can be hidden with:

```toml
[buffer.server_messages.change_host]
enabled = false
```

## Ignore

If dealing with a noisy user or bot, an [ignore filter](../configuration/servers#filters) can be used to hide their messages.  For example, to hide messages produced by `ChanServ` in the `#halloy` channel, the following setting can be used:

```toml
[servers.libera.filters]
ignore = [ { user = "ChanServ", channel = "#halloy" } ]
```
