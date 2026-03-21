# Buffer

Buffer settings for Halloy.

## `line_spacing`

Setting to control spacing between messages in buffers

```toml
# Type: integer
# Values: positive integers
# Default: 0

[buffer]
line_spacing = 4
```

## `scroll_position_on_open`

Scroll position of the buffer when it opens.

```toml
# Type: string
# Values: "oldest-unread", "newest"
# Default: "oldest-unread"

[buffer]
scroll_position_on_open = "newest"
```

## `backlog_separator`

Customize when the backlog separator is displayed within a buffer

### `hide_when_all_read`

Hide backlog divider when all messages in the buffer have been marked as read.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.backlog_separator]
hide_when_all_read = true
```

### `text`

Set the text for backlog divider or disable it

```toml
# Type: boolean or string
# Values: boolean or any string
# Default: true

[buffer.backlog_separator]
text = false
```

## `channel`

Channel specific settings

### `channel_name_casing`

Transform the channel name casing in the channel pane title.

```toml
# Type: string (optional)
# Values: "lowercase"
# Default: not set (channel name displayed as-is)

[buffer.channel]
channel_name_casing = "lowercase"
```

### `message`

Message settings within a channel buffer.

#### `nickname_color`

Nickname colors in the message. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.channel.message]
nickname_color = "unique"
```

#### `show_emoji_reacts`

Whether to display emoji reactions on messages (if [IRCv3 React](https://ircv3.net/specs/client-tags/react) is supported by the server).

```toml
# Type: boolean
# Values: "true", "false"
# Default: "true"

[buffer.channel.message]
show_emoji_reacts = true
```

#### `max_reaction_display`

Maximum number of user-visible characters (Unicode grapheme clusters) in a reaction.
If a reaction exceeds this value, then its display is truncated to the first `max_reaction_display` grapheme clusters.

```toml
# Type: integer
# Values: positive integers
# Default: 5

[buffer.channel.message]
max_reaction_display = 5
```

#### `max_reaction_chars`

Maximum number of user-visible characters (Unicode grapheme clusters) in a reaction.
If a reaction exceeds this value, then it is not stored.

```toml
# Type: integer
# Values: positive integers
# Default: 64

[buffer.channel.message]
max_reaction_chars = 64
```

### `nicklist`

Nicklist settings within a channel buffer.

#### `alignment`

Horizontal alignment of nicknames.

```toml
# Type: string
# Values: "left", "right"
# Default: "left"

[buffer.channel.nicklist]
alignment = "left"
```

#### `away`

Controls the appearance of away nicknames.

```toml
# Type: string or object
# Values: "dimmed", "none" or { dimmed = float }
# Default: "dimmed"
[buffer.channel.nicklist]
away = "dimmed"

# with custom dimming alpha value (0.0-1.0)
[buffer.channel.nicklist]
away = { dimmed = 0.5 }

# no away indication
[buffer.channel.nicklist]
away = "none"
```

#### `color`

Nickname colors in the nicklist. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.channel.nicklist]
color = "unique"
```

#### `enabled`

Control if nicklist should be shown or not by default.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.nicklist]
enabled = true
```

#### `position`

Nicklist position in the pane.

```toml
# Type: string
# Values: "left", "right"
# Default: "left"

[buffer.channel.nicklist]
position = "right"
```

#### `show_access_levels`

Show access level(s) in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: string
# Values: "all", "highest", or "none"
# Default: "highest"

[buffer.channel.nicklist]
show_access_levels = "all"
```

#### `width`

Overwrite nicklist width in pixels.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.channel.nicklist]
width = 150
```

#### `click`

Click action for when interaction with nicknames.

- `"open-query"`: Open a query with the User
- `"insert-nickname"`: Inserts the nickname into text input

```toml
# Type: string
# Values: "open-query", "insert-nickname"
# Default: "open-query"

[buffer.channel.nicklist]
click = "open-query"
```

### `typing`

Typing settings for channel and query buffers.

#### `font_size`

Control the font size of the typing indicator. This also adjusts the bottom padding reserved for the typing indicator line.

```toml
# Type: integer
# Values: positive integers
# Default: not set
# When omitted, Halloy uses the main configured font size.

[buffer.channel.typing]
font_size = 12
```

#### `share`

Control whether Halloy shares your typing status with other users.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.channel.typing]
share = false
```

#### `show`

Control whether Halloy shows typing status from other users.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.typing]
show = true
```

### `topic_banner`

Topic banner settings within a channel buffer.

#### `enabled`

Control if topic banner should be shown or not by default.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.channel.topic_banner]
enabled = true
```

#### `max_lines`

Amount of visible lines before you have to scroll in topic banner.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 2

[buffer.channel.topic_banner]
max_lines = 2
```

## `chathistory`

IRCv3 [`chathistory`](https://ircv3.net/specs/extensions/chathistory) extension settings

### `infinite_scroll`

Automatically request older history when scrolling to the top of a channel/query buffer

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.chathistory]
infinite_scroll = true
```

## `commands`

Commands settings.

### `show_description`

Show or hide the description for a command

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands]
show_description = true
```

### `aliases`

Define custom slash command aliases.

```toml
# Type: map
# Values: map with string key/value pairs
# Default: {}

[buffer.commands.aliases]
op = "/mode #halloy +ooo $1 $2 $3"
halloy = "/me says halloy to $1!"
topic = "/topic #halloy $1-"
deopme = "/mode -o $nick"
np = "/exec mpc current --format '/me is now playing %artist% - %title%'"
```

Use `$1` through `$9` to insert positional arguments.  A hyphen after
the argument number (e.g. `$1-`) means that all following arguments will
also be included in the argument; for example, `/topic our new topic`
for the alias defined as `topic = "/topic #halloy $1-"` will expand to
`/topic #halloy our new topic`.

You can also use context-aware placeholders:

- `$nick` inserts your current nickname.
- `$channel` inserts the active channel name.
- `$server` inserts the active server name.

Aliases must be specified in reference to existing slash commands, so to
send a regular message the `/msg` command (or equivalent, such as
`/plain`/`/format` command) should be used.  For example, `welcome =
"/msg $channel welcome to IRC $1, enjoy your stay!"`.

- Aliases take precedence over built-in commands with the same name.
- Alias expansion happens once; aliases do not expand other aliases.

### `exec`

Configure `/exec`.

::: warning
`/exec` runs a local shell command on your machine. Enable it only if you trust the commands you plan to run.
:::

See the [Exec Command guide](../guides/exec-command.md) for a few simple examples.

#### `enabled`

Enable `/exec`.
When disabled, submitting `/exec` shows an error instead of running the shell command.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.commands.exec]
enabled = false
```

#### `timeout`

Time in seconds to wait before timing out `/exec`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 5

[buffer.commands.exec]
timeout = 5
```

#### `max_output_bytes`

Maximum number of stdout bytes accepted from `/exec`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 4096

[buffer.commands.exec]
max_output_bytes = 4096
```

### `sysinfo`

Configure which system information components to display when using the `/sysinfo` command

#### `cpu`

Show CPU information (processor brand and model)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
cpu = true
```

#### `memory`

Show memory information

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
memory = true
```

#### `gpu`

Show graphics card information (adapter and backend)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
gpu = true
```

#### `os`

Show operating system information (version and kernel)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
os = true
```

#### `uptime`

Show system uptime information

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
uptime = true
```

### `quit`

Configure `QUIT` command

#### `default_reason`

Default quit (from server) reason

```toml
# Type: String
# Values: string value
# Default: ""

[buffer.commands.quit]
default_reason = "See you later all!"
```

### `part`

Configure `PART` command

#### `default_reason`

Default part (from channel) reason

```toml
# Type: String
# Values: string value
# Default: ""

[buffer.commands.part]
default_reason = "I'll be back!"
```

## `date_separators`

Customize how date separators are displayed within a buffer

### `format`

Controls the date format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).  

```toml
# Type: string
# Values: any valid strftime string
# Default: "%A, %B %-d"

[buffer.date_separators]
format = "%A, %B %-d"
```

### `show`

Show date separators.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.date_separators]
show = true
```

## `emojis`

Emojis settings.

```toml
[buffer.emojis]
show_picker = true
skin_tone = "default"
auto_replace = true
```

### `show_picker`

Show the emoji picker when typing `:shortcode:` in text input.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.emojis]
show_picker = true
```

### `skin_tone`

Skin tone selected when picking an emoji.

```toml
# Type: string
# Values: "default", "light", "medium-light", "medium", "medium-dark", "dark"
# Default: "default"

[buffer.emojis]
skin_tone = "default"
```

### `auto_replace`

Automatically replace `:shortcode:` in text input with the corresponding emoji.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.emojis]
auto_replace = true
```

### `characters_to_trigger_picker`

Minimum number of characters after `:` required for the emoji picker to show.
E.g. `:D` will not show the emoji picker unless `characters_to_trigger_picker` is less than or equal to `1`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 2

[buffer.emojis]
characters_to_trigger_picker = 2
```

## `internal_messages`

Internal messages are messages sent from Halloy itself.

### `default`

Default settings which will be used for all internal messages when a specific value is not provided for the specific internal message type.

#### `enabled`

Control if internal messages are enabled by default.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.internal_messages.default]
enabled = true
```

#### `smart`

By default, only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.default]
smart = 180
```

### `error`

Internal messages which are considered an "error" such as when a connection was lost, or when connection to server failed.

#### `enabled`

Control if internal message type is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.internal_messages.error]
enabled = true
```

#### `smart`

Only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.error]
smart = 180
```

### `success`

Internal messages which are considered a "success" such as when a connection was restored, or when connected successfully to a server.

#### `enabled`

Control if internal message type is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.internal_messages.success]
enabled = true
```

#### `smart`

Only show internal message if received within the given time duration (seconds).

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.internal_messages.success]
smart = 180
```

## `mark_as_read`

When to mark a buffer as read

### `on_application_exit`

When exiting the application (all buffers, opened or closed, will be marked as read when the application exits).

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.mark_as_read]
on_application_exit = false
```

### `on_buffer_close`

When closing a buffer (a buffer is considered closed when it is replaced or if it is open when the application exits).  If set to `"scrolled-to-bottom"` then a buffer will only be marked as read if it is scrolled to the bottom when closing (i.e. if the most recent messages are visible).

```toml
# Type: boolean
# Values: true, false, "scrolled-to-bottom"
# Default: "scrolled-to-bottom"

[buffer.mark_as_read]
on_buffer_close = "scrolled-to-bottom"
```

### `on_scroll_to_bottom`

When scrolling to the bottom of a buffer.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.mark_as_read]
on_scroll_to_bottom = true
```

### `on_message_sent`

When sending a message to the buffer.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.mark_as_read]
on_message_sent = true
```

### `on_message`

Marks as read when a new message arrives in the focused buffer and you are at the bottom.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.mark_as_read]
on_message = true
```

## `nickname`

Customize how nicknames are displayed within a buffer.

### `alignment`

Horizontal alignment of nicknames.

```toml
# Type: string
# Values: "left", "right", "top"
# Default: "left"

[buffer.nickname]
alignment = "right"
```

### `away`

Controls the appearance of away nicknames.

```toml
# Type: string or object
# Values: "dimmed", "none" or { dimmed = float }
# Default: "dimmed"
[buffer.nickname]
away = "dimmed"

# with custom dimming alpha value (0.0-1.0)
[buffer.nickname]
away = { dimmed = 0.5 }

# no away indication
[buffer.nickname]
away = "none"
```

### `brackets`

Brackets around nicknames.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.nickname]
brackets = { left = "<", right = ">" }
```

### `color`

Nickname colors in a channel buffer. `"unique"` generates colors by randomizing the hue, while keeping the saturation and lightness from the theme's nickname color.

```toml
# Type: string
# Values: "solid", "unique"
# Default: "unique"

[buffer.nickname]
color = "unique"
```

### `offline`

Controls the appearance of offline nicknames.

```toml
# Type: string or object
# Values: "solid" or "none"
# Default: "solid"
[buffer.nickname]
offline = "solid"

# no offline indication
[buffer.nickname]
offline = "none"
```

### `show_access_levels`

Show access level(s) in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: string
# Values: "all", "highest", or "none"
# Default: "highest"

[buffer.nickname]
show_access_levels = "none"
```

### `shown_status`

What status should be indicated (by either `away` or `offline` settings), the user's current status (`"current"`) or their status at the time of sending the message (`"historical"`).

```toml
# Type: string or object
# Values: "current" or "historical"
# Default: "current"
[buffer.nickname]
shown_status = "current"
```

### `click`

Click action for when interaction with nicknames.

- `"open-query"`: Open a query with the User
- `"insert-nickname"`: Inserts the nickname into text input

```toml
# Type: string
# Values: "open-query", "insert-nickname"
# Default: "open-query"

[buffer.nickname]
click = "open-query"
```

### `truncate`

Truncate nicknames in buffer to a maximum length

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.nickname]
truncate = 10
```

### `hide_consecutive`

Hide nickname if consecutive messages are from the same user.  

::: warning
`hide_consecutive` does not work in conjunction with `alignment = "top"` .
:::

#### `enabled`

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

#### `show_after_previews`

Show nicknames after messages with visible image or link previews.
Note: has no effect when `enabled = false`.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.nickname.hide_consecutive]
show_after_previews = true
```

## server_messages

Server messages are messages sent from an IRC server.

```toml
# Hide all join messages except for #halloy channel:

[buffer.server_messages.join]
exclude = "*"
include = { channels = ["#halloy"] }

# Hide all part messages

[buffer.server_messages.part]
enabled = false
```

### Types

| **Event Type**        | **Description**                                                                                                                |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `change_host`         | Message is sent when a user changes host                                                                                       |
| `change_mode`         | Message is sent when a mode is set                                                                                             |
| `change_nick`         | Message is sent when a user changes nick                                                                                       |
| `change_topic`        | Message is sent when a channel topic is changed                                                                                |
| `join`                | Message is sent when a user joins a channel                                                                                    |
| `kick`                | Message is sent when a user is kicked from a channel                                                                           |
| `monitored_offline`   | Message is sent when a monitored user goes offline                                                                             |
| `monitored_online`    | Message is sent when a monitored user goes online                                                                              |
| `part`                | Message is sent when a user leaves a channel                                                                                   |
| `quit`                | Message is sent when a user closes the connection to a channel or server                                                       |
| `standard_reply_fail` | Message is sent when a command/function fails or an error with the session                                                     |
| `standard_reply_note` | Message is sent when there is information about a command/function or session                                                  |
| `standard_reply_warn` | Message is sent when there is feedback about a command/function or session                                                     |
| `topic`               | Message is sent when the client joins a channel to inform them of the topic (does not include message sent when topic changes) |
| `wallops`             | Message is sent by operators to all users with mode +w on the network                                                          |
| `default`             | Pseudo-type to provide fallback settings for when a specific event type has not been configured                                |

### `enabled`

Control if server message type is enabled.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.server_messages.<server_message>]
enabled = true
```

### `smart`

Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.

```toml
# Type: integer
# Values: any non-negative integer
# Default: not set

[buffer.server_messages.<server_message>]
smart = 180
```

### `exclude`

[Exclusion conditions](/configuration/conditions.md) in which the server message
will be hidden. Inclusion conditions will take precedence over exclusion
conditions. You can also exclude all conditions by setting to `"all"` or `"*"`.

```toml
# Type: inclusion/exclusion conditions
# Values: user, channel, & server inclusion/exclusion conditions
# Default: not set

[buffer.server_messages.<server_message>]
exclude = "*"
```

### `include`

[Inclusion conditions](/configuration/conditions.md) in which the server message
will be shown. Server messages will be shown in all conditions (when enabled)
unless explicitly excluded, so this setting is only relevant when combined with
the `exclude` setting.

```toml
# Type: inclusion/exclusion conditions
# Values: user, channel, & server inclusion/exclusion conditions
# Default: not set

[buffer.server_messages.<server_message>]
include = { channels = ["#halloy"] }
```

### `dimmed`

Dim condensed server message.  Either automatically, based on text/background colors (by setting to `true`), or specify a dimming value in the range `0.0` (transparent) to `1.0` (no dimming).

```toml
# Type: bool or float
# Values: true, false, or float
# Default: true

[buffer.server_messages.<server_message>]
dimmed = true
```

### `username_format`

Adjust the amount of information displayed for a username in server messages. If you choose `"short"`, only the nickname will be shown. If you choose `"full"`, the nickname, username, and hostname (if available) will be displayed.

::: info
Not all server messages uses this setting.
:::

```toml
# Type: string
# Values: "full", "short"
# Default: "full"

[buffer.server_messages.<server_message>]
username_format = "full"
```

### `condense`

Condense multiple consecutive server messages into a single abbreviated message.

#### `messages`

 Message type(s) to condense. Supported types:

| **Event Type** | **Symbol** |
| -------------- | ---------- |
| `change-host`  | `→`        |
| `change-nick`  | `→`        |
| `join`         | `+`        |
| `part`         | `-`        |
| `quit`         | `-`        |
| `kick`         | `!`        |

The color and font style of the symbols is taken from the theme setting for that event type.

```toml
# Type: array of strings
# Values: ["change-host", "change-nick", "join", "kick", "part", "quit"]
# Default: ["change-host", "change-nick", "join", "part", "quit"]

[buffer.server_messages.condense]
messages = ["change-nick", "join", "part", "quit"]
```

#### `dimmed`

Dim condensed messages.  Either automatically, based on text/background colors (by setting to `true`), or specify a dimming value in the range `0.0` (transparent) to `1.0` (no dimming).

```toml
# Type: bool or float
# Values: true, false, or float
# Default: true

[buffer.server_messages.condense]
dimmed = true
```

#### `format`

How to format condensed messages:

- `"brief"`:  Only show changes to channel state.  If a user joins then leaves, then do not show any message.  If a user joins, leaves, then joins again, then show that they joined the channel (`+`).
- `"detailed"`: Include messages that do not change channel state, but do not show repeated events.  If a user joins then leaves, show a condensed message with both events (`+-`).  But, if a user joins and leaves many times in a row, only indicate that they left and re-joined (i.e. still `+-`).
- `"full"`:  Include all messages in the condensed message.  If a user joins and leaves three times, then show a symbol for each event (`+-+-+-`).

```toml
# Type: string
# Values: "brief", "detailed", "full"
# Default: "brief"

[buffer.server_messages.condense]
format = "full"
```

#### `icon`

Marker style for condensed server messages.

```toml
# Type: string
# Values: "none", "chevron", "dot"
# Default: "none"

[buffer.server_messages.condense]
icon = "chevron"
```

## `status_message_prefix`

Status message prefix settings.

### `brackets`

Brackets around status message prefix.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.status_message_prefix]
brackets = { left = "<", right = ">" }
```

## `text_input`

Customize the text input for in buffers.

### `visibility`

Text input visibility. When set to `"focused"` it will only be visible when the buffer is focused.

```toml
# Type: string
# Values: "always", "focused"
# Default: "always"

[buffer.text_input]
visibility = "always"
```

### `auto_format`

Control if the text input should auto format the input. By default text is only formatted when using the `/format` command.

```toml
# Type: string
# Values: "disabled", "markdown", "all"
# Default: "disabled"

[buffer.text_input]
auto_format = "markdown"
```

::: tip
Read more about [text formatting](/guides/text-formatting).
:::

### `key_bindings`

Different key bindings for the text input

```toml
# Type: string
# Values: "default", "emacs"
# Default: "emacs" on macOS, "default" for all other OSes

[buffer.text_input]
key_bindings = "emacs"
```

##### `emacs`

Emacs variant has the following binds:

> `ctrl+a`: Move to the beginning of the line  
  `ctrl+e`: Move to the end of the line  
  `ctrl+b`: Move backward one character  
  `ctrl+f`: Move forward one character  
  `ctrl+d`: Delete the character under the cursor  
  `ctrl+k`: Kill rest of line from cursor  
  `alt+b`: Move the cursor backward one word  
  `alt+f`: Move the cursor forward one word  

::: info
Global [keyboard shortcuts](/configuration/keyboard) take precedence. Unset any that collide (e.g., set `command_bar = "unset"`).
:::

### `max_lines`

Maximum number of lines in a single input.  If [`multiline`](https://ircv3.net/specs/extensions/multiline) is supported by the server then it will be utilized, otherwise messages will be sent individually with [`send_line_delay`](#send_line_delay) milliseconds between them.

::: warning
In many IRC communities sending multiple lines in quick succession is frowned upon (and may be a bannable offense); be mindful of community norms when using this feature
:::

```toml
# Type: integer
# Values: > 0
# Default: 5

[buffer.text_input]
max_lines = 5
```

### `send_line_delay`

Delay (milliseconds) between each line when sending multiple lines.  When the server does not support SAFERATE messages may be delayed longer due to [anti-flood protections](/configuration/servers#anti_flood).

```toml
# Type: integer
# Values: >= 0
# Default: 100

[buffer.text_input]
send_line_delay = 100
```

### `autocomplete`

Customize autocomplete.

#### `order_by`

Ordering that autocomplete uses to select from matching users.

- `"recent"`: Autocomplete users by their last message in the channel;  the user with most recent message autocompletes first, then increasingly older messages.  Users with no seen messages are matched last, in the order specified by `sort_direction`.
- `"alpha"`: Autocomplete users based on alphabetical ordering of potential matches.  Ordering is ascending/descending based on `sort_direction`.

```toml
# Type: string
# Values: "alpha", "recent"
# Default: "recent"

[buffer.text_input.autocomplete]
order_by = "recent"
```

#### `sort_direction`

Sort direction when autocompleting alphabetically.

- `"asc"`: ascending alphabetical (a→z)
- `"desc"`: descending alphabetical (z→a)

```toml
# Type: string
# Values: "asc", "desc"
# Default: "asc"

[buffer.text_input.autocomplete]
sort_direction = "asc"
```

#### `completion_suffixes`

Sets what suffix is added after autocompleting. The first option is for when a nickname is autocompleted at the beginning of a sentence. The second is for when it's autocompleted in the middle of a sentence.

```toml
# Type: array of 2 strings
# Values: array of 2 strings
# Default: [": ", " "]

[buffer.text_input.autocomplete]
completion_suffixes = [": ", " "]
```

### `nickname`

Customize nickname left of text input

#### `enabled`

Display own nickname next to text input field

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.text_input.nickname]
enabled = true
```

#### `show_access_levels`

Show access level(s) in front of nickname (`@`, `+`, `~`, etc.).

```toml
# Type: string
# Values: "all", "highest", or "none"
# Default: "highest"

[buffer.text_input.nickname]
show_access_level = "highest"
```

## `timestamp`

Customize how timestamps are displayed within a buffer.

### `format`

Controls the timestamp format. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).

```toml
# Type: string
# Values: any valid strftime string
# Default: "%R"

[buffer.timestamp]
format = "%R"
```

### `context_menu_format`

Controls the format of shown in a timestamp's context menu. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).

```toml
# Type: string
# Values: any valid strftime string
# Default: "%x"

[buffer.timestamp]
context_menu_format = "%x"
```

### `copy_format`

Controls the format used when copying the timestamp into the clipboard from its context menu. The expected format is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html).  If not set, then the timestamp is copied in the [date and time of day in UTC using extended format ISO 8601:2004(E) 4.3.2 with millisecond precision](https://en.wikipedia.org/wiki/ISO_8601) as is utilized in IRCv3.

```toml
# Type: string
# Values: any valid strftime string or not set
# Default: not set

[buffer.timestamp]
copy_format = "%Y-%m-%d %H:%M:%S"
```

### `brackets`

Brackets around timestamps.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.timestamp]
brackets = { left = "[", right = "]" }
```

### `locale`

Locale used when formatting timestamps, for strftime formats that produce locale-specific output (e.g. `%x`, `%X`, `%a`, etc).  If not specified, then the locale will be set automatically, falling back to the POSIX locale if the system locale cannot be determined.  Supported locales are determined by [`enum Locale` in the `pure-rust-locales` crate](https://docs.rs/pure-rust-locales/latest/pure_rust_locales/enum.Locale.html).

```toml
# Type: string
# Values: IETF BCP 47 language tags
# Default: not set

[buffer.timestamp]
locale = "POSIX"
```

### `hide_consecutive`

Hide timestamp for consecutive messages from the same user.

If specified as `{ smart = integer }` then the timestamp is hidden only when
the previous message is from the same user and sent within `smart` seconds.

```toml
# Type: boolean
# Values: true, false, or { smart = integer }
# Default: false

[buffer.timestamp.hide_consecutive]
enabled = true

# hide if the previous message was from the same user and sent within 2m of the current message
[buffer.timestamp.hide_consecutive]
enabled = { smart = 120 }
```

## `url`

Customize how urls behave in buffers

### `prompt_before_open`

Prompt before opening a hyperlink.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.url]
prompt_before_open = true
```
