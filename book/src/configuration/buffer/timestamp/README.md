# Timestamp

Customize how timestamps are displayed within a buffer.

- [Timestamp](#timestamp)
  - [Configuration](#configuration)
    - [format](#format)
    - [context\_menu\_format](#context_menu_format)
    - [copy\_format](#copy_format)
    - [brackets](#brackets)

## Configuration

### format

Controls the timestamp format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).

```toml
# Type: string
# Values: any valid strftime string
# Default: "%R"

[buffer.timestamp]
format = "%R"
```

### context_menu_format

Controls the format of shown in a timestamp's context menu. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).

```toml
# Type: string
# Values: any valid strftime string
# Default: "%x"

[buffer.timestamp]
context_menu_format = "%x"
```

### copy_format

Controls the format used when copying the timestamp into the clipboard from its context menu. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).  If not set, then the timestamp is copied in the [date and time of day in UTC using extended format ISO 8601:2004(E) 4.3.2 with millisecond precision](https://en.wikipedia.org/wiki/ISO_8601) as is utilized in IRCv3.

```toml
# Type: string
# Values: any valid strftime string or not set
# Default: not set

[buffer.timestamp]
copy_format = "%Y-%m-%d %H:%M:%S"
```

### brackets

Brackets around timestamps.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.timestamp]
brackets = { left = "[", right = "]" }
```
