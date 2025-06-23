# Connect with Soju

To connect with a [**soju**](https://soju.im/) bouncer, the configuration below can be used as a template. Simply change so it fits your credentials.

> 💡  as of 2025.1 Halloy supports [`chathistory`](../configuration/buffer.md#bufferchathistory), so the machine name (like `@desktop`) is no longer needed when `chathistory` is enabled

```toml
[servers.libera]
nickname = "casperstorm"
username = "<your-username>/irc.libera.chat"
server = "irc.squidowl.org"
port = 6697
password = "<your-password>"
chathistory = true
```

You can enable infinite scrolling history as well, if you want to be able to load older messages

```toml
[buffer.chathistory]
infinite_scroll = true
```
