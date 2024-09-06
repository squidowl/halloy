# `[proxy]`

Proxy settings for Halloy.

**Example**

```toml
[proxy]
type = "socks5"
host = "192.168.1.100"
port = 1080
```

## `type`

Proxy type.

- **type**: string
- **values**: `http`, `socks5`
- **default**: not set
   
## `host`

Proxy host to connect to .

- **type**: string
- **values**: any string
- **default**: not set
 
## `port`

Proxy port to connect on.

- **type**: integer
- **values**: any positive integer
- **default**: not set
- 
## `username`

Proxy username (optional).

- **type**: string
- **values**: any string
- **default**: not set
  
## `password`

Proxy password (optional).

- **type**: string
- **values**: any string
- **default**: not set
