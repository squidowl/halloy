# `[proxy]`

Proxy settings for Halloy.

## `type`

Proxy type to use.

```toml
# Type: string
# Values: http, socks5
# Default: not set

[proxy]
type = "socks5"
```

## `host`

Proxy host to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

[proxy]
host = "192.168.1.100"
```
 
## `port`

Proxy port to connect on.

```toml
# Type: integer
# Values: any positive integer
# Default: not set

[proxy]
port = 1080
```
 
## `username`

Proxy username (optional).

```toml
# Type: string
# Values: any string
# Default: not set

[proxy]
username = "username"
```

## `password`

Proxy password (optional).

```toml
# Type: string
# Values: any string
# Default: not set

[proxy]
password = "password"
```
