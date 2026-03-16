# SOCKS5

Socks5 proxy settings.

- [SOCKS5](#socks5)
  - [Example](#example)
  - [Configuration](#configuration)
    - [host](#host)
    - [port](#port)
    - [username](#username)
    - [password](#password)

## Example

```toml
[proxy.socks5]
host = "192.168.1.100"
port = 1080
username = "username"
password = "password"
```

## Configuration

### host

Proxy host to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

# Required

[proxy.socks5]
host = "192.168.1.100"
```

### port

Proxy port to connect on.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

# Required

[proxy.socks5]
port = 1080
```

### username

Proxy username.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.socks5]
username = "username"
```

### password

Proxy password.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.socks5]
password = "password"
```
