# Custom themes

To create a custom theme for Halloy, simply place a theme file (with a `.toml` extension) inside the `themes` folder within the configuration directory.

```toml
# Consider we have a theme called "foobar.toml" inside the themes folder.
# Theme is a root key, so it has to be placed before any sections in your config file.

theme = "foobar"
# .. rest of the configuration file.
```

::: tip
Halloy has a built in theme editor which makes theme creation easier
:::

 Each `"<color string>"` is expected to be a valid hex color. If invalid, or if
 the key is removed, the color will fall back to transparent.

 Each `<text style>` is expected to be either a valid hex color string (`"<color
 string>"`), or table ("`{ color = "<color string>", font_style = "<font style
 string>" }`") with entries for `color` (valid hex color string) and
 `font_style` (valid font style string; `"normal"`, `"italic"`, `"bold"`, or
 `"italic-bold"`).

 A custom theme is structured as follows:

```toml
[general]
background = "<color string>"
border = "<color string>"
horizontal_rule = "<color string>"
scrollbar = "<color string>"
unread_indicator = "<color string>"
highlight_indicator = "<color string>"

[text]
primary = <text style>
secondary = <text style>
tertiary = <text style>
success = <text style>
error = <text style>
warning = <text style>
info = <text style>
debug = <text style>
trace = <text style>

[buttons.primary]
background = "<color string>"
background_hover = "<color string>"
background_selected = "<color string>"
background_selected_hover = "<color string>"

[buttons.secondary]
background = "<color string>"
background_hover = "<color string>"
background_selected = "<color string>"
background_selected_hover = "<color string>"

[buffer]
action = <text style>
background = "<color string>"
background_text_input = "<color string>"
background_title_bar = "<color string>"
border = "<color string>"
border_selected = "<color string>"
code = <text style>
highlight = "<color string>"
nickname = <text style>
nickname_offline = <text style>
selection = "<color string>"
timestamp = <text style>
topic = <text style>
url = <text style>
backlog_rule = "<color string>"

[buffer.server_messages]
# Set below if you want to have a unique color for each.
# Otherwise simply set `default` to use that for all server messages.
#
# change_host = <text style>
# change_mode = <text style>
# change_nick = <text style>
# change_topic = <text style>
# join = <text style>
# kick = <text style>
# part = <text style>
# quit = <text style>
# topic = <text style>
# monitored_online = <text style>
# monitored_offline = <text style>
# standard_reply_fail = <text style>
# standard_reply_warn = <text style>
# standard_reply_note = <text style>
# wallops = <text style>
default = <text style>

[formatting]
# Set below if you want override the default color used in formatted messages.
#
# white = "<color string>"
# black = "<color string>"
# blue = "<color string>"
# green = "<color string>"
# red = "<color string>"
# brown = "<color string>"
# magenta = "<color string>"
# orange = "<color string>"
# yellow = "<color string>"
# lightgreen = "<color string>"
# cyan = "<color string>"
# lightcyan = "<color string>"
# lightblue = "<color string>"
# pink = "<color string>"
# grey = "<color string>"
# lightgrey = "<color string>"
```

More information on formatting colors is available in the [text formatting guide](/guides/text-formatting.md).

::: info
The default Ferra theme toml file can be viewed [on GitHub](https://github.com/squidowl/halloy/blob/main/assets/themes/ferra.toml).
:::

## Base16

The [base16](https://github.com/chriskempson/base16) color scheme framework
includes hundreds of color schemes build using 16 colors. These color schemes have
are compiled for Halloy in the
[`4e554c4c/base16-halloy`](https://github.com/4e554c4c/base16-halloy)
repository.

To use these themes, download `themes.tar.gz` from the
[latest release](https://github.com/4e554c4c/base16-halloy/releases/latest)
and unpack it to the `themes` folder in the Halloy configuration directory. Then
you can enable themes individually in `config.toml`.

### Example

```toml
# Static
theme = "base16-gruvbox-dark-hard"
```
