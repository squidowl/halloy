# `[buffer.date_separators]`

Customize how date separators are displayed within a buffer.

**Example**

```toml
[buffer.date_separators]
format = "%A, %B %-d"
```

## `format`

Controls the date format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).  
NOTE: The application will panic if a invalid format is provided.

- **type**: string
- **values**: any string
- **default**: `"%A, %B %-d"`

## `show`

Show date separators. 

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`