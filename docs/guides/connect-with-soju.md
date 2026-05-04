# Connect with soju

Halloy supports two different ways to connect with a [**soju**](https://soju.im/) bouncer

1. Automatic network detection using the [`bouncer-networks`](https://codeberg.org/emersion/soju/src/branch/master/doc/ext/bouncer-networks.md) extension
2. Manual per-network configuration (legacy)

::: tip
It is strongly recommended to [configure](https://soju.im/doc/soju.1.html#CONFIG_FILE) soju with the `db` `message-store` driver with:
```
message-store db
```

Both the `fs` and `memory` `message-store` drivers are incompatible with the proper functioning of some IRCv3 features (e.g. IRCv3 `chathistory` with the `memory` driver or IRCv3 `reactions` with the `fs` driver).
:::

## Automatic network detection using the `bouncer-networks` extension

To connect using the `bouncer-networks` extension, you can use the following configuration template. This will ensure you are automatically connected to all your networks.

```toml
[servers.<name>]
nickname = "<your-nickname>"
server = "<your-bouncer-url>"

[servers.<name>.sasl.plain]
username = "<your-username>"
password = "<your-password>"
```

If you haven't configured any networks beforehand, you can do so after connecting. Note that you might need to restart Halloy to see newly created networks in the sidebar.

```sh
/msg BouncerServ net create -addr irc.libera.chat
```


## Manual per-network configuration (legacy)

If you would rather manually connect to each server, you can use the following configuration template.

```toml
[servers.<name>]
nickname = "<your-nickname>"
server = "<your-bouncer-url>"
port = 6697

[servers.<name>.sasl.plain]
username = "<your-username>/<network>"
password = "<your-password>"
```

Here is an example configuration for connecting to Libera:

```toml
[servers.libera]
nickname = "casperstorm"
server = "irc.your-bouncer-url.org"

[servers.libera.sasl.plain]
username = "casperstorm/irc.libera.chat"
password = "my-password"
```

## Using Chat History

You can enable infinite scrolling history if you want to automatically load older messages.

```toml
[buffer.chathistory]
infinite_scroll = true
```
