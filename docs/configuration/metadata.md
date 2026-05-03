# Metadata

IRCv3 metadata settings.

Metadata support depends on the server advertising the `draft/metadata-2`
capability.

## `preferred_keys`

Metadata keys to subscribe to, in order of preference.

```toml
# Type: array of strings
# Values: "display-name", "avatar", "pronouns", "homepage", "color", "status"
# Default: ["display-name", "avatar", "pronouns", "homepage", "color", "status"]

[metadata]
preferred_keys = ["display-name", "avatar", "pronouns", "homepage", "color", "status"]
```

## `avatar`

Avatar settings.

### `enabled`

Enable or disable loading avatars.

```toml
# Type: boolean
# Values: true, false
# Default: true

[metadata.avatar]
enabled = true
```

### `exclude`

Exclude avatar URLs from loading by providing regex patterns.

```toml
# Type: array of strings
# Values: array of regex patterns
# Default: []

[metadata.avatar]
exclude = []
```

Prevent avatars from loading for matching URLs:

::: tip
Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This allows you to write the regex without escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.

Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`
:::

```toml
[metadata.avatar]
exclude = [
    '''https?://(www\.)?example\.com/.*''',
]
```
