# HTTP

Http proxy settings.

- [HTTP](#http)
  - [Example](#example)
  - [Configuration](#configuration)
    - [host](#host)
    - [port](#port)
    - [username](#username)
    - [password](#password)

## Example

```toml
[proxy.http]
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

[proxy.http]
host = "192.168.1.100"
```

### port

Proxy port to connect on.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

# Required

[proxy.http]
port = 1080
```

### username

Proxy username.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.http]
username = "username"
```

### password

Proxy password.

```toml
# Type: string
# Values: any string
# Default: not set

# Optional

[proxy.http]
password = "password"
```
