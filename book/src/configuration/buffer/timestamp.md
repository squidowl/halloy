# `[buffer.timestamp]`

Customize how timestamps are displayed within a buffer.

**Example**

```toml
[buffer.timestamp]
format = "%R"
brackets = { left = "[", right = "]" }
```

## `format`

Controls the timestamp format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).

- **type**: string
- **values**: any string
- **default**: `"%R"`

## `brackets`

Brackets around timestamps. 

- **type**: object
- **values**: `{ left = "<any string>", right = "<any string>" }`
- **default**: `{ left = "", right = "" }`