# `[buffer.text_input]`

Customize the text input for in buffers.

**Example**

```toml
[buffer.text_input]
visibility = "always"
auto_format = "markdown"
```

## `visibility`

Text input visibility. When set to `"focused"` it will only be visible when the buffer is focused.

- **type**: string
- **values**: `"always"`, `"focused"`
- **default**: `"always"`

## `auto_format`

Control if the text input should auto format the input. By default text is only formatted when using the `/format` command (Read more: [Text Formatting](../../guides/text-formatting.html)).

- **type**: string
- **values**: `"disabled"`, `"markdown"`, `"all"`
- **default**: `"disabled"`