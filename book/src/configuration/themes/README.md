# Themes

- [Themes](#themes)
  - [Example](#example)
  - [Configuration](#configuration)
    - [theme](#theme)
  - [Custom themes](#custom-themes)
  - [Sub-sections](#sub-sections)
    - [Community](#community)
    - [Base16](#base16)


## Example

```toml
# Static single
theme = "ferra"

# Static multiple (random selection)
theme = ["ferra", "booberry"]

# Dynamic single
theme = { light = "ferra-light", dark = "ferra" }

# Dynamic multiple (random selection)
theme = { light = ["ferra-light", "booberry-light"], dark = ["ferra", "booberry"] }
```

## Configuration

> ⚠️  `theme` is a root key, so it must be placed before every section.

### theme

Specify the theme name(s) to use. The theme must correspond to a file in the `themes` folder of your Halloy configuration directory. For more details, see the [configuration overview](../../configuration.md). The default theme in Halloy is [Ferra](https://github.com/casperstorm/ferra/).

When multiple themes are specified, Halloy will randomly select one each time the application starts. When a dynamic theme is used, Halloy will match the appearance of the OS.

- **type**: string, array of strings, or object
- **values**: `"<string>"`, `["<string>", "<string>"]`, `{ light = "<string>", dark = "<string>" }`, `{ light = ["<string>", "<string>"], dark = ["<string>", "<string>"] }`
- **default**: `"ferra"`
  
> 💡  See all [community created themes](./community.md) and [base16 themes](./base16.md).

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
highlight_indicator = "<string>"

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
nickname_offline = "<string>"
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
# kick = "<string>"
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

> 💡  The default Ferra theme toml file can be viewed [on GitHub](https://github.com/squidowl/halloy/blob/main/assets/themes/ferra.toml).


## Sub-sections

### [Community](community.md)

Community created themes for Halloy

### [Base16](base16.md)

Community collection of base16 themes
