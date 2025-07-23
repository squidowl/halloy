# Themes

## Example

```toml
# Static
theme = "ferra"

# Dynamic
theme = { light = "ferra-light", dark = "ferra" }
```

> ⚠️  `theme` is a root key, so it must be placed before every section.

## `theme`

Specify the theme name(s) to use. The theme must correspond to a file located in the `themes` folder, which can be found in the Halloy configuration directory. The default theme in Halloy is [Ferra](https://github.com/casperstorm/ferra/).

When a dynamic theme is used, Halloy will match the appearance of the OS.

- **type**: string or object
- **values**: `"<string>"`, `{ light = "<string>", dark = "<string>" }`
- **default**: `"ferra"`
  
> 💡  See all community created themes [here](./community.md) and base16 themes [here](./base16.md).

## Custom themes

To create a custom theme for Halloy, simply place a theme file (with a `.toml` extension) inside the `themes` folder within the configuration directory.

```toml
# Consider we have a theme called "foobar.toml" inside the themes folder.
# Theme is a root key, so it has to be placed before any sections in your config file.

theme = "foobar"
# .. rest of the configuration file.
```

> 💡  Halloy has a built in theme editor which makes theme creation easier

 Each `"<string>"` is expected to be a valid hex color. If invalid, or if the key is removed, the color will fallback to transparent. A custom theme is structured as follows:

```toml
[general]
background = "<string>"
border = "<string>"
horizontal_rule = "<string>"
scrollbar = "<string>"
unread_indicator = "<string>"

[text]
primary = "<string>"
secondary = "<string>"
tertiary = "<string>"
success = "<string>"
error = "<string>"
warning = "<string>"
info = "<string>"
debug = "<string>"
trace = "<string>"

[buttons.primary]
background = "<string>"
background_hover = "<string>"
background_selected = "<string>"
background_selected_hover = "<string>"

[buttons.secondary]
background = "<string>"
background_hover = "<string>"
background_selected = "<string>"
background_selected_hover = "<string>"

[buffer]
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

[buffer.server_messages]
# Set below if you want to have a unique color for each.
# Otherwise simply set `default` to use that for all server messages.
#
# change_host = "<string>"
# change_mode = "<string>"
# change_nick = "<string>"
# join = "<string>"
# part = "<string>"
# quit = "<string>"
# reply_topic = "<string>"
# monitored_online = "<string>"
# monitored_offline = "<string>"
# standard_reply_fail = "<string>"
# standard_reply_warn = "<string>"
# standard_reply_note = "<string>"
# wallops = "<string>"
default = "<string>"
```
> 💡  The default Ferra theme toml file can be viewed [here](https://github.com/squidowl/halloy/blob/main/assets/themes/ferra.toml).
