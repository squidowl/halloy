# Text Input

Customize the text input for in buffers.

- [Text Input](#text-input)
  - [Configuration](#configuration)
    - [visibility](#visibility)
    - [auto\_format](#auto_format)
  - [Sub-sections](#sub-sections)
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

## Sub-sections

### [Autocomplete](autocomplete.md)

Customize autocomplete

### [Nickname](nickname.md)

Customize nickname left of text input
