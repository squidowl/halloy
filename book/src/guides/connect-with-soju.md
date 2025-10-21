# Connect with Soju

To connect with a [**soju**](https://soju.im/) bouncer, the configuration below can be used as a template. Simply change so it fits your credentials.

```toml
[servers.soju]
nickname = "<your-nickname>"
server = "<your-bouncer-url>"
port = 6697
[servers.soju.sasl.plain]
username = "<your-username>"
password = "<your-password>"
```

> 💡  as of 2025.1 Halloy supports [`chathistory`](../configuration/servers/index.md#chathistory), so the machine name (like `@desktop`) is no longer needed when `chathistory` is enabled

## Using bouncer networks

> ⚠️  The soju bouncer networks specification *requires* that SASL be used. If you do not use SASL, you must add servers in the legacy fashion.

As of 2025.9 Halloy supports [`bouncer networks`](https://codeberg.org/emersion/soju/src/branch/master/doc/ext/bouncer-networks.md) so connecting to individual servers is no longer needed. Instead, Halloy can communicate with soju to determine what networks you are currently connected to and then automatically add them in the UI.

If this is not desired, you can still add individual servers with the ZNC username syntax, for example:

```toml
[servers.libera]
nickname = "<your-nickname>"
server = "<your-bouncer-url>"
port = 6697

[servers.soju.sasl.plain]
username = "<your-username>/irc.libera.chat"
password = "<your-password>"
```

## Using chathistory

You can enable infinite scrolling history if you want to automatically load older messages.

```toml
[buffer.chathistory]
infinite_scroll = true
```
