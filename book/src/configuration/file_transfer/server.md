# `[file_transfer.server]`

This section is **required** if `passive = false`. One side of the file transfer must
operate as the "server", who the other user connects with to establish a connection.

**Example**

```toml
[file_transfer.server]
public_address = "<some ip>"
bind_address = "<some ip>"
bind_port_first = 1024
bind_port_last = 5000
```

## `public_address`

Address advertised to the remote user to connect to.

- **type**: string
- **values**: any string
- **default**: not set
 
## `public_address`

Address to bind to when accepting connections.

- **type**: string
- **values**: any string
- **default**: not set

## `bind_port_first`

First port in port range to bind to.

- **type**: integer
- **values**: any positive integer
- **default**: not set

## `bind_port_last`

Last port in port range to bind to.

- **type**: integer
- **values**: any positive integer
- **default**: not set