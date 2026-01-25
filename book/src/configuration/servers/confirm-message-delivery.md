# Confirm Message Delivery

Whether and where to confirm delivery of sent messages, if the server supports [`echo-message`](https://ircv3.net/specs/extensions/echo-message)

- [Confirm Message Delivery](#confirm-message-delivery)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [exclude](#exclude)
    - [include](#include)

## Configuration

### enabled

Control if delivery of sent messages is to be confirmed (if the server supports [`echo-message`](https://ircv3.net/specs/extensions/echo-message)).

```toml
# Type: boolean
# Values: true, false
# Default: true

[servers.<name>.confirm_message_delivery]
enabled = true
```

### exclude

[Exclusion conditions](/configuration/conditions.md) in which sent message
delivery confirmation will be skipped. Inclusion conditions will take precedence
over exclusion conditions. You can also exclude all conditions by setting to
`"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: user & channel inclusion/exclusion conditions
# Default: not set

[servers.<name>.confirm_message_delivery]
exclude = "*"
```

### include

[Inclusion conditions](/configuration/conditions.md) in which sent message
delivery will be confirmed . Delivery of sent messages be confirmed in all
conditions (when enabled) unless explicitly excluded, so this setting is only
relevant when combined with the `exclude` setting.

```toml
# Type: inclusion/exclusion conditions
# Values: user & channel inclusion/exclusion conditions
# Default: not set

[servers.<name>.confirm_message_delivery]
include = { channels = ["#halloy"] }
```
