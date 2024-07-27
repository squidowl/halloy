# Buffer

## `[buffer]` Section

## `[buffer.nickname]` Section

### `[buffer.nickname.color]` Section

```toml
[buffer.nickname.color]
kind = "unique" | "solid"
hex = "<string>"
```

| Key    | Description                                                                                                                                                                                                                                                 | Default           |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| `kind` | Controls whether nickname color is `"solid"` or `"unique"`. `"unique"` generates colors by randomzing a hue which is used together with the saturation and lightness from the action color provided by the theme. This color can be overwritten with `hex`. | `kind = "unique"` |
| `hex`  | Overwrite the default color. Optional.                                                                                                                                                                                                                      | `not set`         |

### `[buffer.nickname.brackets]` Section

```toml
[buffer.nickname.brackets]
left = "<string>"
right = "<string>"
```

| Key     | Description                  | Default      |
| ------- | ---------------------------- | ------------ |
| `left`  | Left bracket for nicknames.  | `left = ""`  |
| `right` | Right bracket for nicknames. | `right = ""` |

## `[buffer.timestamp]` Section

```toml
[buffer.timestamp]
format = "<string>"
brackets = { left = "<string>", right = "<string>" }
```

| Key        | Description                                                                                                                                  | Default                     |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| `format`   | Format expected is [strftime](https://pubs.opengroup.org/onlinepubs/007908799/xsh/strftime.html). To disable, simply pass empty string `""`. | `"%R"`                      |
| `brackets` | Brackets for nicknames                                                                                                                       | `{ left = "", right = "" }` |

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
```

| Key        | Description                                      | Default   |
| ---------- | ------------------------------------------------ | --------- |
| `enabled`  | Control if nicklist should be shown or not       | `true`    |
| `position` | Nicklist position. Can be `"left"` or `"right"`. | `"right"` |

### `[buffer.channel.nicklist.color]` Section

```toml
[buffer.channel.nicklist.color]
kind = "unique" | "solid"
hex = "<string>"
```

| Key    | Description                                                                                                                                                                                                                                                 | Default           |
| ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| `kind` | Controls whether nickname color is `"solid"` or `"unique"`. `"unique"` generates colors by randomzing a hue which is used together with the saturation and lightness from the action color provided by the theme. This color can be overwritten with `hex`. | `kind = "unique"` |
| `hex`  | Overwrite the default color. Optional.                                                                                                                                                                                                                      | `not set`         |

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
hex = "<string>"
```

```toml
[buffer.server_messages.part]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
hex = "<string>"
```

```toml
[buffer.server_messages.quit]
enabled = true | false
smart = <integer>
username_format = "full" | "short"
hex = "<string>"
```

```toml
[buffer.server_messages.topic]
enabled = true | false
hex = "<string>"
```

| Key               | Description                                                                                                                                                      | Default   |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `enabled`         | Control if the server message should appear in buffers or not                                                                                                    | `true`    |
| `smart`           | Only show server message if the user has sent a message in the given time interval (seconds) prior to the server message.                                        | `not set` |
| `username_format` | Adjust how the username should look. Can be `"full"` (shows the longest username available (nickname, username and hostname) or `"short"` (only shows nickname). | `"full"`  |
| `hex`             | Overwrite the default color. Optional.                                                                                                                           | `not set` |

## `[buffer.internal_messages]` Section

```toml
[buffer.internal_messages.success]
enabled = true | false
smart = <integer>
hex = "<string>"
```

```toml
[buffer.internal_messages.error]
enabled = true | false
smart = <integer>
hex = "<string>"
```

| Key       | Description                                                                      | Default   |
| --------- | -------------------------------------------------------------------------------- | --------- |
| `enabled` | Control if the internal message should appear in buffers or not                  | `true`    |
| `smart`   | Only show internal message if received within the given time duration (seconds). | `not set` |
| `hex`     | Overwrite the default color. Optional.                                           | `not set` |
