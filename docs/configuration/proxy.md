# Proxy

Proxy settings for Halloy.

> 💡 [Preview](/configuration/preview) requests will be routed through the same proxy that the corresponding message is routed through (i.e. if a proxy is configured for a server, then all previews for messages on that server will be routed through the proxy).  Except for the for the [Tor](#tor) proxy;  when utilizing the Tor proxy preview requests are disabled.

## `http`

Http proxy settings.

### `host`

Proxy host to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

# Required

[proxy.http]
host = "192.168.1.100"
```

### `port`

Proxy port to connect on.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

# Required

[proxy.http]
port = 1080
```

### `username`

Proxy username.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.http]
username = "username"
```

### `password`

Proxy password.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.http]
password = "password"
```

## `socks5`

Socks5 proxy settings.

### `host`

Proxy host to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

# Required

[proxy.socks5]
host = "192.168.1.100"
```

### `port`

Proxy port to connect on.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

# Required

[proxy.socks5]
port = 1080
```

### `username`

Proxy username.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.socks5]
username = "username"
```

### `password`

Proxy password.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.socks5]
password = "password"
```

## `tor`

Tor proxy settings. Utilizes [Arti](https://arti.torproject.org/) to integrate Tor support directly into Halloy. Does not integrate into a pre-existing Tor setup.  To utilize an existing Tor daemon, use [`[proxy.socks5]`](#socks5) instead.

It accepts no further configuration.

> ⚠️ Tor support is **not included by default**. You must build Halloy with the `tor` feature to use this proxy type. See [Optional Features](../guides/optional-features.md) for build instructions.

> ⚠️ To preserve privacy, [previews](/configuration/preview) are disabled when using the Tor proxy.

### Example

```toml
[proxy.tor]
```
