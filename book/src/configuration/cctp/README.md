# CTCP

[Client-to-Client Protocol](https://modern.ircdocs.horse/ctcp) response settings.

- [CTCP](#ctcp)
  - [Example](#example)
  - [Configuration](#configuration)
    - [ping](#ping)
    - [source](#source)
    - [time](#time)
    - [version](#version)
    - [userinfo](#userinfo)


## Example

```toml
# Disable responses for TIME and VERSION responses

[ctcp]
time = false
version = false
```

## Configuration

### ping

Whether Halloy will respond to a [CTCP PING](https://modern.ircdocs.horse/ctcp#ping) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
ping = true
```

### source

Whether Halloy will respond to a [CTCP SOURCE](https://modern.ircdocs.horse/ctcp#source) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
source = true
```

### time

Whether Halloy will respond to a [CTCP TIME](https://modern.ircdocs.horse/ctcp#time) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
time = true
```

### version

Whether Halloy will respond to a [CTCP VERSION](https://modern.ircdocs.horse/ctcp#version) message.

```toml
# Type: boolean
# Values: true, false
# Default: true

[ctcp]
version = true
```

### userinfo

Whether Halloy will respond to a [CTCP USERINFO](https://modern.ircdocs.horse/ctcp#userinfo) message. The response is enabled if this option is set to a string, which will be used as the reply.

```toml
# Type: string
# Values: any string
# Default: not set

# Example usage: <nickname> (<realname>)
# KVIrc usage:   Age=<age>; Gender=<gender>; Location=<location>; Languages=<languages>; <other>

[ctcp]
userinfo = "<nickname> (<realname>)"
```
