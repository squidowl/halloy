# Text Input

Customize the text input for in buffers.

## visibility

Text input visibility. When set to `"focused"` it will only be visible when the buffer is focused.

```toml
# Type: string
# Values: "always", "focused"
# Default: "always"

[buffer.text_input]
visibility = "always"
```

## auto_format

Control if the text input should auto format the input. By default text is only formatted when using the `/format` command.

```toml
# Type: string
# Values: "disabled", "markdown", "all"
# Default: "disabled"

[buffer.text_input]
auto_format = "markdown"
```

> 💡 Read more about [text formatting](../../../guides/text-formatting.md).

## key_bindings

Different key bindings for the text input

```toml
# Type: string
# Values: "default", "emacs"
# Default: "emacs" on macOS, "default" for all other OSes

[buffer.text_input]
key_bindings = "emacs"
```

#### emacs

Emacs variant has the following binds:

> `ctrl+a`: Move to the beginning of the line  
  `ctrl+e`: Move to the end of the line  
  `ctrl+b`: Move backward one character  
  `ctrl+f`: Move forward one character  
  `ctrl+d`: Delete the character under the cursor  
  `ctrl+k`: Kill rest of line from cursor  
  `alt+b`: Move the cursor backward one word  
  `alt+f`: Move the cursor forward one word  

> 💡 Global [keyboard shortcuts](../../keyboard.md) take precedence. Unset any that collide (e.g., set `command_bar = "unset"`).

## max_lines

Maximum number of lines in a single input.  If [`multiline`](https://ircv3.net/specs/extensions/multiline) is supported by the server then it will be utilized, otherwise messages will be sent individually with [`send_line_delay`](#send_line_delay) milliseconds between them.

> ⚠️ In many IRC communities sending multiple lines in quick succession is frowned upon (and may be a bannable offense); be mindful of community norms when using this feature

```toml
# Type: integer
# Values: > 0
# Default: 5

[buffer.text_input]
max_lines = 5
```

## send_line_delay

Delay (milliseconds) between each line when sending multiple lines.  When the server does not support SAFERATE messages may be delayed longer due to [anti-flood protections](/configuration/servers/#anti_flood).

```toml
# Type: integer
# Values: >= 0
# Default: 100

[buffer.text_input]
send_line_delay = 100
```
