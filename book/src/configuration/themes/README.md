# Themes

```toml
theme = "<string>"
```

| Key         | Description                  | Default  |
| ----------- | ---------------------------- | -------- |
| `theme`[^1] | Name of the theme to use[^2] | `""`[^3] |

[^1]: `theme` is a root key, so it must be placed before any section.
[^2]: Name of theme file inside `themes` folder.
[^3]: Using [Ferra](https://github.com/casperstorm/ferra/) by default.

## Custom themes

To create a custom theme for Halloy, simply place a theme file (with a `.toml` extension) inside the themes folder within the configuration directory.

> 💡  The configuration direction can be found [here](../../configuration/).

 Each `"<string>"` is expected to be a valid hex color. If invalid, or if the key is removed, the color will fallback to transparent. A custom theme is structured as follows:

```toml
[colors.general]
background = "<string>"
border = "<string>"
horizontal_rule = "<string>"
unread_indicator = "<string>"

[colors.text]
primary = "<string>"
secondary = "<string>"
tertiary = "<string>"
success = "<string>"
error = "<string>"

[colors.buttons.primary]
background = "<string>"
background_hover = "<string>"
background_selected = "<string>"
background_selected_hover = "<string>"

[colors.buttons.secondary]
background = "<string>"
background_hover = "<string>"
background_selected = "<string>"
background_selected_hover = "<string>"

[colors.buffer]
action = "<string>"
background = "<string>"
background_text_input = "<string>"
background_title_bar = "<string>"
border = "<string>"
border_selected = "<string>"
code = "<string>"
highlight = "<string>"
nickname = "<string>"
selection = "<string>"
timestamp = "<string>"
topic = "<string>"
url = "<string>"

[colors.buffer.server_messages]
# Set below if you want to have a unique color for each.
# Otherwise simply set `default` to use that for all server messages.
#
# change_host = "<string>"
# join = "<string>"
# part = "<string>"
# quit = "<string>"
# reply_topic = "<string>"
default = "<string>"
```
> 💡  The default Ferra theme toml file can be viewed [here](https://github.com/squidowl/halloy/blob/main/assets/themes/ferra.toml).
