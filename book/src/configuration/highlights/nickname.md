# Nickname

Nickname highlights

- [Nickname](#nickname)
  - [Example](#example)
  - [Configuration](#configuration)
    - [exclude](#exclude)
    - [include](#include)
    - [case\_insensitive](#case_insensitive)

## Example

```toml
# Enable nickname highlights only in channel #halloy.
[highlights.nickname]
exclude = ["*"]
include = ["#halloy"]
```

## Configuration

### exclude

Channels in which you won’t be highlighted.
If you pass `["#halloy"]`, you won’t be highlighted in that channel. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[highlights.nickname]
exclude = ["*"]
```

### include

Channels in which you will be highlighted, only useful when combined with `exclude = ["*"]`.
If you pass `["#halloy"]`, you will only be highlighted in that channel.

```toml
# Type: array of strings
# Values: array of any strings
# Default: ["*"]

[highlights.nickname]
exclude = ["*"]
include = ["#halloy"]
```

### case_insensitive

Whether or not to trigger regardless nickname highlight regardless of case.
Uses the casemapping [specified by server](https://modern.ircdocs.horse/#casemapping-parameter).

```toml
# Type: boolean
# Values: true, false
# Default: true

[highlights.nickname]
case_insensitive = false
```
