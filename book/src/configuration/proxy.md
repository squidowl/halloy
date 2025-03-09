# `[proxy]`

Proxy settings for Halloy.

1. [http](#proxyhttp)
2. [socks5](#proxysocks5)
3. [tor](#proxytor)

## `[proxy.http]`

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
# Values: any positive integer
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

## Example 

```toml
[proxy.http]
host = "192.168.1.100"
port = 1080
username = "username"
password = "password"
```

## `[proxy.socks5]`

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
# Values: any positive integer
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

## Example 

```toml
[proxy.socks5]
host = "192.168.1.100"
port = 1080
username = "username"
password = "password"
```

## `[proxy.tor]`

Tor proxy settings. Utilizes the [arti](https://arti.torproject.org/) to integrate Tor natively.
It accepts no further configuration.

## Example 

```toml
[proxy.tor]
```