# Config editor

Settings for the config editor.

## `indent`

Text inserted when Tab is pressed in the config editor. `"tab"` inserts a tab character. A positive integer from 1 to 8 inserts that many spaces.

```toml
# Type: string or integer
# Values: "tab", or an integer from 1 to 8
# Default: "tab"

[config_editor]
indent = "tab"
```

Insert four spaces:

```toml
[config_editor]
indent = 4
```
