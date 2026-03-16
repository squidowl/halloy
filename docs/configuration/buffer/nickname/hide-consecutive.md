# Hide Consecutive

Hide nickname if consecutive messages are from the same user.  

- [Hide\_Consecutive](#hide-consecutive)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [show_after_previews](#show_after_previews)


## Configuration

> ⚠️ `hide_consecutive` does not work in conjunction with `alignment = "top"` .


### enabled

If specified as `{ smart = integer }` then the nickname will be hidden for consecutive messages
are from the same user and each is within `smart` seconds of each other.

```toml
# Type: boolean
# Values: true, false, or { smart = integer }
# Default: false

[buffer.nickname.hide_consecutive]
enabled = true

# hide if the previous message was from the same user and sent within 2m of the current message
[buffer.nickname.hide_consecutive]
enabled = { smart = 120 }
```

### show_after_previews

Show nicknames after messages with visible image or link previews.
Note: has no effect when `enabled = false`.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.nickname.hide_consecutive]
show_after_previews = true