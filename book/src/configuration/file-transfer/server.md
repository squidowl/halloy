# Server

This section is **required** if [passive is true](./#passive). One side of the file transfer must
operate as the "server", who the other user connects with to establish a connection.

- [Server](#server)
  - [Configuration](#configuration)
    - [public\_address](#public_address)
    - [bind\_address](#bind_address)
    - [bind\_port\_first](#bind_port_first)
    - [bind\_port\_last](#bind_port_last)

## Configuration

### public_address

Address advertised to the remote user to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer.server]
public_address = "<some ip>"
```

### bind_address

Address to bind to when accepting connections.

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer.server]
bind_address = "<some ip>"
```

### bind_port_first

First port in port range to bind to.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[file_transfer.server]
bind_port_first = 1024
```

### bind_port_last

Last port in port range to bind to.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[file_transfer.server]
bind_port_last = 5000
```
