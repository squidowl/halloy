# `[ctcp]`

[Client-to-Client Protocol](https://modern.ircdocs.horse/ctcp) response settings.

**Example**

```toml
# Disable responses for TIME and VERSION responses

[ctcp]
time = false
version = false
```

# `ping`

Whether Halloy will respond to a [CTCP PING](https://modern.ircdocs.horse/ctcp#ping) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
ping = true
```

# `source`

Whether Halloy will respond to a [CTCP TIME](https://modern.ircdocs.horse/ctcp#source) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
source = true
```

# `time`

Whether Halloy will respond to a [CTCP TIME](https://modern.ircdocs.horse/ctcp#time) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
time = true
```

# `version`

Whether Halloy will respond to a [CTCP VERSION](https://modern.ircdocs.horse/ctcp#version) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
version = true
```
