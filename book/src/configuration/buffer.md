# Buffer

## `[buffer]` Section

## `[buffer.nickname]` Section

```toml
[buffer.nickname]
alignment = "left" | "right" 
color = "unique" | "solid"
brackets = { left = "<string>", right = "<string>" }
```

| Key         | Description                                                                                                                                                                                                         | Default                     |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| `alignment` | Alignment option for nicknames in buffer.                                                                                                                                                                           | `"left"`                    |
| `color`     | Controls whether nickname color is `"solid"` or `"unique"`. `"unique"` generates colors by randomzing a hue which is used together with the saturation and lightness from the nickname color provided by the theme. | `"unique"`                  |
| `brackets`  | Brackets for nicknames.                                                                                                                                                                                             | `{ left = "", right = "" }` |



## `[buffer.timestamp]` Section

```toml
[buffer.timestamp]
format = "<string>"
brackets = { left = "<string>", right = "<string>" }
```

| Key        | Description                                                                                                                                  | Default                     |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| `format`   | Format expected is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html). To disable, simply pass empty string `""`. | `"%R"`                      |
| `brackets` | Brackets for timestamps.                                                                                                                     | `{ left = "", right = "" }` |

```toml
[buffer.status_message_prefix]
brackets = { left = "<string>", right = "<string>" }
```

| Key        | Description                                      | Default                     |
| ---------- | ------------------------------------------------ | --------------------------- |
| `brackets` | Brackets for status message prefixes (uncommon). | `{ left = "", right = "" }` |

## `[buffer.text_input]` Section

```toml
[buffer.text_input]
visibility = "always" | "focused"
auto_format = "disabled" | "markdown" | "all"
```

| Key           | Description                                              | Default      |
| ------------- | -------------------------------------------------------- | ------------ |
| `visibility`  | Text input visibility. Can be `"always"` or `"focused"`. | `"always"`   |
| `auto_format` | Auto format text without using `format` command.         | `"disabled"` |

## `[buffer.channel]` Section

### `[buffer.channel.nicklist]` Section

```toml
[buffer.channel.nicklist]
enabled = true | false
position = "left" | "right"
color = "unique" | "solid"
```

| Key        | Description                                                                                                                                                                                                         | Default    |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| `enabled`  | Control if nicklist should be shown or not                                                                                                                                                                          | `true`     |
| `position` | Nicklist position. Can be `"left"` or `"right"`.                                                                                                                                                                    | `"right"`  |
| `color`    | Controls whether nickname color is `"solid"` or `"unique"`. `"unique"` generates colors by randomzing a hue which is used together with the saturation and lightness from the nickname color provided by the theme. | `"unique"` |



### `[buffer.channel.topic]` Section

```toml
[buffer.channel.topic]
enabled = true | false
max_lines = <integer>
```

| Key         | Description                                                        | Default |
| ----------- | ------------------------------------------------------------------ | ------- |
| `enabled`   | Control if topic banner should be shown or not                     | `false` |
| `max_lines` | Amount of visible lines before you have to scroll in topic banner. | `2`     |

## `[buffer.server_messages]` Section

```toml
[buffer.server_messages.join]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.part]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.quit]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
```

```toml
[buffer.server_messages.topic]
enabled = true | false
```

```toml
[buffer.server_messages.change_host]
enabled = true | false
smart = <integer>
```

| Key               | Description                                                                                                                                                      | Default   |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `enabled`         | Control if the server message should appear in buffers or not                                                                                                    | `true`    |
| `smart`           | Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.                                        | `not set` |
| `username_format` | Adjust how the username should look. Can be `"full"` (shows the longest username available (nickname, username and hostname) or `"short"` (only shows nickname). | `"full"`  |

## `[buffer.internal_messages]` Section

```toml
[buffer.internal_messages.success]
enabled = true | false
smart = <integer>
```

```toml
[buffer.internal_messages.error]
enabled = true | false
smart = <integer>
```

| Key       | Description                                                                      | Default   |
| --------- | -------------------------------------------------------------------------------- | --------- |
| `enabled` | Control if the internal message should appear in buffers or not                  | `true`    |
| `smart`   | Only show internal message if received within the given time duration (seconds). | `not set` |
