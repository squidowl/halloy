# File Transfer

File transfer configuration options.

## enabled

Control if file transfers are shown in the UI (e.g. file transfer requests are
shown, file transfer options are presented in menus, etc).

```toml
# Type: boolean
# Values: true, false
# Default: true

[file_transfer]
enabled = true
```

## save_directory

Default directory to save files in. If not set, user will see a file dialog. [^1]

```toml
# Type: string
# Values: any string
# Default: not set

[file_transfer]
save_directory = "/Users/halloy/Downloads"
```

## passive

If true, act as the "client" for the transfer. Requires the remote user act as the [server](./server.md).

```toml
# Type: boolean
# Values: true, false
# Default: true

[file_transfer]
passive = true
```

## timeout

Time (in seconds) to wait before timing out a transfer waiting to be accepted.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 300

[file_transfer]
timeout = 300
```
