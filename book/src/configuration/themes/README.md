# Themes

```toml
theme = "<string>"
```

| Key     | Description                  | Default  |
| ------- | ---------------------------- | -------- |
| `theme`[^1] | Name of the theme to use[^2] | `""`[^3] |

[^1]: `theme` is a root key, so it must be placed before any section.
[^2]: Name of theme file inside `themes` folder.
[^3]: Using [Ferra](https://github.com/casperstorm/ferra/) by default.

## Custom themes

To create a custom theme for Halloy, simply place a theme file (with a `.toml` extension) inside the themes folder within the configuration directory.

> 💡  The configuration direction can be found [here](../../configuration/).

A custom theme is structured as follows.

```toml
name = "<string>"

[palette]
background = "<string>"
text = "<string>"
action = "<string>"
accent = "<string>"
alert = "<string>"
error = "<string>"
info = "<string>"
success = "<string>"
```

| Key       | Description                                       |
| --------- | ------------------------------------------------- |
| `name`    | Name of the theme to use                          |
| `palette` | Colors expect a hex color string. Eg: `"#2b292d"` |
