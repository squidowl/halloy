# `[file_transfer]`

File transfer configuration options.

## `save_directory`

Default directory to save files in. If not set, user will see a file dialog.

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer]
save_directory = "/Users/halloy/Downloads"
```

## `passive`

If true, act as the "client" for the transfer. Requires the remote user act as the [server](#file_transferserver).

```toml
# Type: boolean
# Values: true, false
# Default: true

[file_transfer]
passive = true
```

## `timeout`

Time (in seconds) to wait before timing out a transfer waiting to be accepted.

```toml
# Type: integer
# Values: any positive integer
# Default: 300

[file_transfer]
timeout = 300
```

# `[file_transfer.server]`

This section is **required** if `passive = false`. One side of the file transfer must
operate as the "server", who the other user connects with to establish a connection.

## `public_address`

Address advertised to the remote user to connect to.

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer.server]
public_address = "<some ip>"
```

## `bind_address`

Address to bind to when accepting connections.

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer.server]
bind_address = "<some ip>"
```

## `bind_port_first`

First port in port range to bind to.

```toml
# Type: integer
# Values: any positive integer
# Default: not set

[file_transfer.server]
bind_port_first = 1024
```

## `bind_port_last`

Last port in port range to bind to.

```toml
# Type: integer
# Values: any positive integer
# Default: not set

[file_transfer.server]
bind_port_last = 5000
```

# `[file_transfer.auto_accept]`

Configuration for automatically accepting incoming file transfers.

## `enabled`

If true, automatically accept incoming file transfers. Requires `save_directory` to be set.

```toml
# Type: boolean
# Values: true, false
# Default: false

[file_transfer.auto_accept]
enabled = false
```

## `nicks`

If true, automatically accept incoming file transfers from these nicks.
Note `auto_accept` has to be enabled.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[file_transfer.auto_accept]
nicks = ["nick1", "nick2"]
```

## `masks`

If true, automatically accept incoming file transfers from these nicks. Matches are made against the full nickname (i.e. nickname, username, and hostname in the format `nickname!username@hostname`). Note `auto_accept` has to be enabled.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[file_transfer.auto_accept]
masks = [
    '''nick!ident@example\.com''',
    '''.*@foobar\.com'''
]
```