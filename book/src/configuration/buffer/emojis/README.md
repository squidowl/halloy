# Emojis

Emojis settings.

- [Emojis](#emojis)
  - [Example](#example)
  - [Configuration](#configuration)
    - [show\_picker](#show_picker)
    - [skin\_tone](#skin_tone)
    - [auto\_replace](#auto_replace)
    - [characters\_to\_trigger\_picker](#characters_to_trigger_picker)

## Example

```toml
[buffer.emojis]
show_picker = true
skin_tone = "default"
auto_replace = true
```

## Configuration

### show_picker

Show the emoji picker when typing `:shortcode:` in text input.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.emojis]
show_picker = true
```

### skin_tone

Skin tone selected when picking an emoji.

```toml
# Type: string
# Values: "default", "light", "medium-light", "medium", "medium-dark", "dark"
# Default: "default"

[buffer.emojis]
skin_tone = "default"
```

### auto_replace

Automatically replace `:shortcode:` in text input with the corresponding emoji.

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.emojis]
auto_replace = true
```

### characters_to_trigger_picker

Minimum number of characters after `:` required for the emoji picker to show.
E.g. `:D` will not show the emoji picker unless `characters_to_trigger_picker` is less than or equal to `1`.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 2

[buffer.emojis]
characters_to_trigger_picker = 2
```
