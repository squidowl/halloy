# `[file_transfer]`

File transfer configuration options.

**Example**

```toml
[file_transfer]
save_directory = "$HOME/Downloads"
passive = true
timeout = 180
```

## `save_directory`

Default directory to open when prompted to save a file.

- **type**: string
- **values**: any string
- **default**: `"$HOME/Downloads"`

## `passive`

If true, act as the "client" for the transfer. Requires the remote user act as the [server](#file_transferserver-section).

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `timeout`

Time (in seconds) to wait before timing out a transfer waiting to be accepted.

- **type**: integer
- **values**: any positive integer
- **default**: `300`
