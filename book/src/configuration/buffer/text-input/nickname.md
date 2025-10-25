# Nickname

Customize nickname left of text input

- [Nickname](#nickname)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [show\_access\_level](#show_access_level)

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

### show_access_level

Show access levels in front of nickname (`@`, `+`, `~`, etc.).

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.text_input.nickname]
show_access_level = true
```
