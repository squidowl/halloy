# `[proxy]`

Proxy settings for Halloy.

**Example**

```toml
[proxy]
Socks5 = { host = "192.168.1.100", port = 1080 }
```

## `Http`

Utilizes an HTTP proxy.

- **host**: string
- **port**: u16
- **username**: string (optional)
- **password**: string (optional)

## `Socks5`

Utilizes a SOCKS5 proxy.

- **host**: string
- **port**: u16
- **username**: string (optional)
- **password**: string (optional)

## `Tor`

Utilizes the [arti](https://arti.torproject.org) to integrate Tor natively.

It accepts no further configuration.