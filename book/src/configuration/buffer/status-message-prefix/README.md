# Status Message Prefix

Status message prefix settings.

- [Status Message Prefix](#status-message-prefix)
  - [Configuration](#configuration)
  - [brackets](#brackets)

## Configuration

## brackets

Brackets around status message prefix.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.status_message_prefix]
brackets = { left = "<", right = ">" }
```
