# SASL Plain

Plain SASL auth using a username and password

- [SASL Plain](#sasl-plain)
  - [Configuration](#configuration)
    - [username](#username)
    - [password](#password)
    - [password\_file](#password_file)
    - [password\_file\_first\_line\_only](#password_file_first_line_only)
    - [password\_command](#password_command)

## Configuration

### username

The account name used for authentication.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
username = "username"
```

### password

The password associated with the account used for authentication.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password = "password"
```

### password_file

Read `password` from the file at the given path.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password_file = ""
```

### password_file_first_line_only

Read `password` from the first line of `password_file` only.

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>]
password_file_first_line_only = true
```

### password_command

Executes the command with `sh` (or equivalent) and reads `password` as the output.

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.plain]
password_command = ""
```

[^1]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
[^2]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).
