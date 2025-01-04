## `[sasl.plain]`

**Example**

```toml
[servers.liberachat.sasl.plain]
username = "foobar"
password = "barbaz"
```

## `username`

The account name used for authentication.

- **type**: string
- **values**: any string
- **default**: not set

## `password`

The password associated with the account used for authentication.

- **type**: string
- **values**: any string
- **default**: not set

## `password_file`

Read `password` from the file at the given path.[^1] [^2]

- **type**: string
- **values**: any string
- **default**: not set

## `password_command`

Executes the command with `sh` (or equivalent) and reads `password` as the output.

- **type**: string
- **values**: any string
- **default**: not set

[^1]: Shell expansions (e.g. `"~/"` → `"/home/user/"`) are not supported in path strings.
[^2]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
