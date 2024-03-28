# File Transfer

## `[file_transfer]` Section

```toml
[file_transfer]
passive = <bool>
timeout = <integer>
```

| Key                | Description                                                                 | Default      |
| ----------------   | --------------------------------------------------------------------------- | ------------ |
| `passive`          | ..                                                                          | `true`       |
| `timeout`          | Time in seconds to wait before timing out a transfer waiting to be accepted | `300`        |


## `[file_transfer.bind]` Section

```toml
[file_transfer.bind]
address = "<string>"
port_first = <integer>
port_last = <integer>

```
| Key              | Description                                   | Default |
| ---------------- | --------------------------------------------- | ------- |
| `address`        | Address to bind to when accepting connections | `""`    |
| `port_first`     | First port in port range to bind to           | `""`    |
| `port_last`      | Last port in port range to bind to            | `""`    |

