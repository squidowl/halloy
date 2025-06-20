# `[font]`

Application wide font settings.

> ⚠️  Changes to font settings require an application restart to take effect.

> 💡  If Halloy is unable to load the specified font & weight, an fallback font may be used.  If the font looks wrong, double-check the family name and that the font family has the specified weight.

## `family`

Monospaced font family to use.

```toml
# Type: string
# Values: any string
# Default: not set
#
# Note: Iosevka Term is provided by the application, and used by default.

[font]
family = "Comic Mono"
```

## `size`

Font size.

```toml
# Type: integer
# Values: any positive integer
# Default: 13

[font]
size = 13
```

## `weight`

Font weight.

```toml
# Type: string
# Values: "thin", "extra-light", "light", "normal", "medium", "semibold", "bold", "extra-bold", and "black"
# Default: "normal"

[font]
weight = "light"
```

## `bold-weight`

Bold font weight.  If not set, then the font weight three steps above the regular font weight (e.g. font weight `"light"` → bold font weight `"semibold"`).

```toml
# Type: string
# Values: "thin", "extra-light", "light", "normal", "medium", "semibold", "bold", "extra-bold", and "black"
# Default: not set

[font]
bold-weight = "semibold"
```
