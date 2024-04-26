# Proxy

## `[proxy]` Section

Example

```toml
[proxy]
type = "socks5"
host = "<string>"
port = <integer>
```

| Key        | Description                                       | Default     |
| :--------- | :------------------------------------------------ | :---------- |
| `type`     | Proxy type. Only `socks5` is currently supported. | `""`        |
| `host`     | Proxy host to connect to                          | `""`        |
| `port`     | Proxy port to connect on                          | `""`        |
| `username` | Proxy username, optional                          | `""`        |
| `password` | Proxy password, optional                          | `""`        |
