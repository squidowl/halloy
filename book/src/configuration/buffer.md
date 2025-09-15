# `[buffer]`

Buffer settings for Halloy.

1. [Channel](#bufferchannel) - Channel specific settings
   1. [Message](#bufferchannelmessage) - Message settings within a channel buffer
   2. [Nicklist](#bufferchannelnicklist) - Nicklist settings within a channel buffer
   3. [Topic](#bufferchanneltopic) - Topic settings within a channel buffer
2. [Chathistory](#bufferchathistory) - IRCv3 Chat History extension settings
3. [Commands](#buffercommands) - Commands settings
4. [Backlog Separator](#bufferbacklog_separator) - Customize when the backlog separator is displayed within a buffer
4. [Date Separators](#bufferdate_separators) - Customize how date separators are displayed within a buffer
5. [Emojis](#bufferemojis) - Emojis settings
6. [Internal Messages](#bufferinternal_messages) - Internal messages are messages sent from Halloy itself
7. [Mark as Read](#buffermark_as_read) - When to automatically mark a buffer as read
8. [Nickname](#buffernickname) - Customize how nicknames are displayed within a buffer
9. [Server Messages](#bufferserver_messages) - Server messages are messages sent from an IRC server.
10. [Status Message Prefix](#bufferstatus_message_prefix) - Status message prefix settings
11. [Text Input](#buffertext_input) - Customize the text input for in buffers
12. [Timestamp](#buffertimestamp) - Customize how timestamps are displayed within a buffer
13. [Url](#bufferurl) - URLs in buffers
14. [Line Spacing](#line_spacing) - Line spacing in buffers

## `[buffer.channel]`

Channel specific settings

### `[buffer.channel.message]`

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

### `[buffer.channel.nicklist]`

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

Show access levels in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.channel.nicklist]
show_access_levels = true
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

### `[buffer.channel.topic]`

Topic settings within a channel buffer.

#### `enabled`

Control if topic should be shown or not by default.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.channel.topic]
enabled = true
```

#### `max_lines`

Amount of visible lines before you have to scroll in topic banner.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 2

[buffer.channel.topic]
max_lines = 2
```

## `[buffer.chathistory]`

IRCv3 Chat History extension settings

### `infinite_scroll`

Automatically request older history when scrolling to the top of a channel/query buffer

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.chathistory]
infinite_scroll = true
```

## `[buffer.commands]`

Commands settings.

```toml
[buffer.commands]
show_description = false
```

### `show_description`

Show or hide the description for a command

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands]
show_description = true
```

## `[buffer.backlog_separator]`

Customize when the backlog separator is displayed within a buffer

### `hide_when_all_read`

Hide backlog divider when all messages in the buffer have been marked as read.

```toml
# Type: boolean
# Values: true, false
# Default: false

[buffer.buffer_separator]
hide_when_all_read = true
```

## `[buffer.date_separators]`

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

## `[buffer.emojis]`

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

## `[buffer.internal_messages]`

Internal messages are messages sent from Halloy itself.

### `[buffer.internal_messages.success]`

A internal messages which is considered a "success" such as when a connection was restored, or when connected successfully to a server.

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

### `[buffer.internal_messages.error]`

A internal messages which is considered a "error" such as when a connection was lost, or when connection to server failed.

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

## `[buffer.mark_as_read]`

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

## `[buffer.nickname]`

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

Show access levels in front of nicknames (`@`, `+`, `~`, etc.).

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.nickname]
show_access_levels = true
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

## `[buffer.server_messages]`

Server messages are messages sent from an IRC server.

| **Event Type**        | **Description**                                                               |
| --------------------- | ----------------------------------------------------------------------------- |
| `change_host`         | Message is sent when a user changes host                                      |
| `change_mode`         | Message is sent when a mode is set                                            |
| `change_nick`         | Message is sent when a user changes nick                                      |
| `change_topic`        | Message is sent when a channel topic is changed                               |
| `join`                | Message is sent when a user joins a channel                                   |
| `kick`                | Message is sent when a user is kicked from a channel                          |
| `monitored_offline`   | Message is sent when a monitored user goes offline                            |
| `monitored_online`    | Message is sent when a monitored user goes online                             |
| `part`                | Message is sent when a user leaves a channel                                  |
| `quit`                | Message is sent when a user closes the connection to a channel or server      |
| `standard_reply_fail` | Message is sent when a command/function fails or an error with the session    |
| `standard_reply_note` | Message is sent when there is information about a command/function or session |
| `standard_reply_warn` | Message is sent when there is feedback about a command/function or session    |
| `topic`               | Message is sent when the client joins a channel to inform them of the topic   |

Example

```toml
# Hide all join messages except for `#halloy` channel:

[buffer.server_messages.join]
exclude = ["*"]
include = ["#halloy"]

# Disable all part messages

[buffer.server_messages.part]
enabled = false
```

### `enabled`

Control if internal message type is enabled.

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

Exclude channels from receiving the server message.
If you pass `["#halloy"]`, the channel `#halloy` will not receive the server message. You can also exclude all channels by using a wildcard: `["*"]`.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[buffer.server_messages.<server_message>]
exclude = ["*"]
```

### `include`

Include channels to receive the server message.
If you pass `["#halloy"]`, the channel `#halloy` will receive the server message. The include rule takes priority over exclude, so you can use both together. For example, you can exclude all channels with `["*"]` and then only include a few specific channels.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[buffer.server_messages.<server_message>]
include = ["#halloy"]
```

### `username_format`

Adjust the amount of information displayed for a username in server messages. If you choose `"short"`, only the nickname will be shown. If you choose `"full"`, the nickname, username, and hostname (if available) will be displayed.

Note: Not all server messages uses this setting.

```toml
# Type: string
# Values: "full", "short"
# Default: "full"

[buffer.server_messages.<server_message>]
username_format = "full"
```

## `[buffer.status_message_prefix]`

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

## `[buffer.text_input]`

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

> 💡 Read more about [text formatting](../guides/text-formatting.md).

### `[buffer.text_input.autocomplete]`

Customize autocomplete.

#### `order_by`

Ordering that autocomplete uses to select from matching users.

- `"recent"`: Autocomplete users by their last message in the channel;  the user with most recent message autocompletes first, then increasingly older messages.  Users with no seen messages are matched last, in the order specified by `sort_direction`.
- `"alpha"`: Autocomplete users based on alphabetical ordering of potential matches.  Ordering is asecnding/descinding based on `sort_direction`.

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

## `[buffer.timestamp]`

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

### `brackets`

Brackets around timestamps.

```toml
# Type: string
# Values: { left = "<any string>", right = "<any string>" }
# Default: { left = "", right = "" }

[buffer.timestamp]
brackets = { left = "[", right = "]" }
```

## `[buffer.url]`

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

## `[line_spacing]`

Setting to control spacing between messages in buffers

```toml
# Type: integer
# Values: positive integers
# Default: 0

[buffer]
line_spacing = 4
```
