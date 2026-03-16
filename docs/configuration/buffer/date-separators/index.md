# Date Separators

Customize how date separators are displayed within a buffer

- [Date Separators](#date-separators)
  - [Configuration](#configuration)
    - [format](#format)
    - [show](#show)

## Configuration

### format

Controls the date format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).  

```toml
# Type: string
# Values: any valid strftime string
# Default: "%A, %B %-d"

[buffer.date_separators]
format = "%A, %B %-d"
```

### show

Show date separators.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.date_separators]
show = true
```
