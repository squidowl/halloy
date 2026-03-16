# Nickname

Customize nickname left of text input

- [Nickname](#nickname)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [show\_access\_levels](#show_access_levels)

## Configuration

### enabled

Display own nickname next to text input field

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.text_input.nickname]
enabled = true
```

### show_access_levels

Show access level(s) in front of nickname (`@`, `+`, `~`, etc.).

```toml
# Type: string
# Values: "all", "highest", or "none"
# Default: "highest"

[buffer.text_input.nickname]
show_access_level = "highest"
```
