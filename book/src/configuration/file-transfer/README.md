# File Transfer

File transfer configuration options.

- [File Transfer](#file-transfer)
  - [Configuration](#configuration)
    - [save\_directory](#save_directory)
    - [passive](#passive)
    - [timeout](#timeout)
  - [Auto Accept](#auto-accept)
  - [Server](#server)

## Configuration

### save_directory

Default directory to save files in. If not set, user will see a file dialog. [^1]

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer]
save_directory = "/Users/halloy/Downloads"
```

### passive

If true, act as the "client" for the transfer. Requires the remote user act as the [server](./server.md).

```toml
# Type: boolean
# Values: true, false
# Default: true

[file_transfer]
passive = true
```

### timeout

Time (in seconds) to wait before timing out a transfer waiting to be accepted.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 300

[file_transfer]
timeout = 300
```

## [Auto Accept](auto_accept.md)

Configure automatic acceptance of incoming file transfers

## [Server](server.md)

Server configuration for file transfers (required when `passive = true`)

[^1]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).
