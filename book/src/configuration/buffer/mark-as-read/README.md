# Mark as Read

When to mark a buffer as read

- [Mark as Read](#mark-as-read)
  - [Configuration](#configuration)
    - [on\_application\_exit](#on_application_exit)
    - [on\_buffer\_close](#on_buffer_close)
    - [on\_scroll\_to\_bottom](#on_scroll_to_bottom)
    - [on\_message\_sent](#on_message_sent)

## Configuration

### on_application_exit

When exiting the application (all buffers, opened or closed, will be marked as read when the application exits).

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.mark_as_read]
on_application_exit = false
```

### on_buffer_close

When closing a buffer (a buffer is considered closed when it is replaced or if it is open when the application exits).  If set to `"scrolled-to-bottom"` then a buffer will only be marked as read if it is scrolled to the bottom when closing (i.e. if the most recent messages are visible).

```toml
# Type: boolean
# Values: true, false, "scrolled-to-bottom"
# Default: "scrolled-to-bottom"

[buffer.mark_as_read]
on_buffer_close = "scrolled-to-bottom"
```

### on_scroll_to_bottom

When scrolling to the bottom of a buffer.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.mark_as_read]
on_scroll_to_bottom = true
```

### on_message_sent

When sending a message to the buffer.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.mark_as_read]
on_message_sent = true
```
