# Backlog Separator

Customize when the backlog separator is displayed within a buffer

- [Backlog Separator](#backlog-separator)
  - [Configuration](#configuration)
    - [hide\_when\_all\_read](#hide_when_all_read)
    - [text](#text)

## Configuration

### hide_when_all_read

Hide backlog divider when all messages in the buffer have been marked as read.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.backlog_separator]
hide_when_all_read = true
```

### text

Set the text for backlog divider or disable it

```toml
# Type: boolean or string
# Values: boolean or any string
# Default: true

[buffer.backlog_separator]
text = false
```
