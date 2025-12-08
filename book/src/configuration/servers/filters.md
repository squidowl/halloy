# Filters

Filter messages based on various criteria.

- [Filters](#filters)
  - [Configuration](#configuration)
    - [ignore](#ignore)
    - [regex](#regex)

## Configuration

### ignore

A list of users to ignore. Users may be identified in any of these four ways:

- A string of the exact nickname to ignore in all contexts (equivalent nicknames, as defined by the server's [casemapping](https://modern.ircdocs.horse/#casemapping-parameter), will be ignored).
- A user & channel pair, written as `{ user = "nickname", channel = "#channel" }`, to ignore the user only in the specified channel.
- A regular expression, written as `{ regex = "pattern" }`, where any user whose nickname matches the regular expression will be ignored.
- A regular expression & channel pair, written as `{ regex = "pattern", channel = "#channel" }`, where any user whose nicknames matches the regular expression will be ignored in the specified channel.

```toml
# Type: array of user identifiers
# Values: array of any user identifiers
# Default: not set

[servers.<name>.filters]
ignore = [
"ignored_user", 
{ regex = '''(?i)ignored_users-.*''' },
{ user = "user_in_channel", channel = "#channel_with_user" },
{ regex = '''(?i)users_in_channel-.*''', channel = "#channel_with_users" }
]
```

### regex

A list of regex used to filter messages; if a match is found in the message text, then the message will be hidden.

```toml
# Type: array of strings
# Values: array of any strings
# Default: not set

[servers.<name>.filters]
regex = [
'''(?i)\bunwanted_pattern\b''',
'''(?i)^[A-Z ]+$''',
]
```
