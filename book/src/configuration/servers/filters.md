# Filters

Filter messages based on various criteria.

- [Filters](#filters)
  - [Configuration](#configuration)
    - [ignore](#ignore)

## Configuration

### ignore

A list of nicknames to ignore. Optionally, the nickname may be preceded by a channel name like so: `"#channel nickname"` - this will ignore the nickname for a specific channel only.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>.filters]
ignore = [
"ignored_user", 
"another_user",
"#specific-channel user_only_for_channel"
]
```
