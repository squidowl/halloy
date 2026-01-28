# Font

Application wide font settings.

- [Font](#font)
  - [Configuration](#configuration)
    - [family](#family)
    - [size](#size)
    - [line_height](#line_height)
    - [weight](#weight)
    - [bold_weight](#bold_weight)
    - [only_emojis_size](#only_emojis_size)

> ⚠️  Changes to font settings require an application restart to take effect.

> 💡  If Halloy is unable to load the specified font & weight, an fallback font may be used.  If the font looks wrong, double-check the family name and that the font family has the specified weight.


## Configuration

### family

Monospaced font family to use.

> ⚠️ Variable-weight fonts are not currently supported.

```toml
# Type: string
# Values: any string
# Default: not set
#
# Note: Iosevka Term is provided by the application, and used by default.

[font]
family = "Comic Mono"
```

### size

Font size.

```toml
# Type: integer
# Values: any positive integer
# Default: 13

[font]
size = 13
```

### line_height

Line height (relative to the font size).

```toml
# Type: number
# Values: any positive float
# Default: not set (resolves to LineHeight::default()) in iced, which is 1.3

[font]
line_height = 1.1
```

### weight

Font weight.

```toml
# Type: string
# Values: "thin", "extra-light", "light", "normal", "medium", "semibold", "bold", "extra-bold", and "black"
# Default: "normal"

[font]
weight = "light"
```

### bold_weight

Bold font weight.  If not set, then the font weight three steps above the regular font weight (e.g. font weight `"light"` → bold font weight `"semibold"`).

```toml
# Type: string
# Values: "thin", "extra-light", "light", "normal", "medium", "semibold", "bold", "extra-bold", and "black"
# Default: not set

[font]
bold_weight = "semibold"
```

### only_emojis_size

Font size for messages that contain only emojis.  If not set, then the regular font size will be used.

```toml
# Type: integer
# Values: any positive integer
# Default: not set

[font]
only_emojis_size = 18
```
