# Themes

## `[theme]` Section

```toml
theme = "<string>"
```


| Key     | Description                  | Default  |
| ------- | ---------------------------- | -------- |
| `theme` | Name of the theme to use[^1] | `""`[^2] |


[^1]: Name of theme file inside `themes` folder 
[^2]: Using [Ferra](https://github.com/casperstorm/ferra/) by default.

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
