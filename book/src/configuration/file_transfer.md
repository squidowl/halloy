# File Transfer

## `[file_transfer]` Section

```toml
[file_transfer]
save_directory = "<string>"
passive = true | false
timeout = <integer>
```

| Key                | Description                                                                                                               | Default           |
| ----------------   | ----------------------------------------------------------------------------------------------------------------          | ----------------- |
| `save_directory`   | Directory opened when prompted to save a file                                                                             | `$HOME/Downloads` |
| `passive`          | If true, act as the "client" for the transfer. Requires the remote user act as the [server](#file_transferserver-section) | `true`            |
| `timeout`          | Time (in seconds) to wait before timing out a transfer waiting to be accepted                                             | `300`             |

## `[file_transfer.server]` Section

This section is **required** if `passive = false`. One side of the file transfer must
operate as the "server", who the other user connects with to establish a connection.

```toml
[file_transfer.server]
public_address = "<string>"
bind_address = "<string>"
bind_port_first = <integer>
bind_port_last = <integer>
```
| Key               | Description                                         | Default |
| ----------------  | ---------------------------------------------       | ------- |
| `public_address`  | Address advertised to the remote user to connect to | `""`    |
| `bind_address`    | Address to bind to when accepting connections       | `""`    |
| `bind_port_first` | First port in port range to bind to                 | `""`    |
| `bind_port_last`  | Last port in port range to bind to                  | `""`    |

