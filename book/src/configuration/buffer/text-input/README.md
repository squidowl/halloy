# Text Input

Customize the text input for in buffers.

- [Text Input](#text-input)
  - [Configuration](#configuration)
    - [visibility](#visibility)
    - [auto\_format](#auto_format)
    - [key\_bindings](#key_bindings)
      - [emacs](#emacs)
  - [Autocomplete](#autocomplete)
  - [Nickname](#nickname)

## Configuration

### visibility

Text input visibility. When set to `"focused"` it will only be visible when the buffer is focused.

```toml
# Type: string
# Values: "always", "focused"
# Default: "always"

[buffer.text_input]
visibility = "always"
```

### auto_format

Control if the text input should auto format the input. By default text is only formatted when using the `/format` command.

```toml
# Type: string
# Values: "disabled", "markdown", "all"
# Default: "disabled"

[buffer.text_input]
auto_format = "markdown"
```

> 💡 Read more about [text formatting](../../../guides/text-formatting.md).

### key_bindings

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

## [Autocomplete](autocomplete.md)

Customize autocomplete

## [Nickname](nickname.md)

Customize nickname left of text input
