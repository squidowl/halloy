# Exec

Configure `/exec`

- [Exec](#exec)
  - [Configuration](#configuration)
    - [timeout](#timeout)
    - [max_output_bytes](#max_output_bytes)

## Configuration

### timeout

Time in seconds to wait before timing out `/exec`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[buffer.commands.exec]
timeout = 5
```

### max_output_bytes

Maximum number of stdout bytes accepted from `/exec`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 4096

[buffer.commands.exec]
max_output_bytes = 4096
```
