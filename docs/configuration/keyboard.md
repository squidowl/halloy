<!-- markdownlint-disable MD033 -->

# Keyboard

Customize keyboard shortcuts. Below is a list of all actions which can be mapped.

```toml
[keyboard]
move_up = ["alt+up", "alt+k"]
move_down = ["alt+down", "alt+j"]
move_left = ["alt+left", "alt+h"]
move_right = ["alt+right", "alt+l"]
quit_application = "alt+q"
command_bar = "noop"
```

Note you can disable a key bind by setting it to `"noop"` or `"none"`.
Each shortcut accepts either a single keybind string or an array of keybind strings.

## Types

| Key                            | Description                         | Default MacOS                                       | Default Other                                       |
| ------------------------------ | ----------------------------------- | --------------------------------------------------- | --------------------------------------------------- |
| `move_up`                      | Moves focus up                      | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>↑</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>↑</kbd>     |
| `move_down`                    | Moves focus down                    | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>↓</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>↓</kbd>     |
| `move_left`                    | Moves focus left                    | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>←</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>←</kbd>     |
| `move_right`                   | Moves focus right                   | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>→</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>→</kbd>     |
| `new_horizontal_buffer`        | New horizontal buffer               | None                                                | None                                                |
| `new_vertical_buffer`          | New vertical buffer                 | None                                                | None                                                |
| `close_buffer`                 | Close focused buffer                | <kbd>⌘</kbd> + <kbd>w</kbd>                         | <kbd>ctrl</kbd> + <kbd>w</kbd>                      |
| `maximize_buffer`              | Maximize focused buffer             | <kbd>⌘</kbd> + <kbd>shift</kbd> + <kbd>↑</kbd>      | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>↑</kbd>   |
| `restore_buffer`               | Restore focused buffer              | <kbd>⌘</kbd> + <kbd>shift</kbd> + <kbd>↓</kbd>      | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>↓</kbd>   |
| `cycle_next_buffer`            | Cycle to next buffer                | <kbd>ctrl</kbd> + <kbd>tab</kbd>                    | <kbd>ctrl</kbd> + <kbd>tab</kbd>                    |
| `cycle_previous_buffer`        | Cycle to previous buffer            | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>tab</kbd> | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>tab</kbd> |
| `cycle_next_unread_buffer`     | Cycle to next buffer                | <kbd>ctrl</kbd> + <kbd>`</kbd>                      | <kbd>ctrl</kbd> + <kbd>`</kbd>                      |
| `cycle_previous_unread_buffer` | Cycle to previous buffer            | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>`</kbd>   | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>`</kbd>   |
| `scroll_up_page`               | Scroll buffer up a page             | <kbd>Fn</kbd> + <kbd>↑</kbd>                        | <kbd>pageup</kbd>                                   |
| `scroll_down_page`             | Scroll buffer down a page           | <kbd>Fn</kbd> + <kbd>↓</kbd>                        | <kbd>pagedown</kbd>                                 |
| `scroll_to_top`                | Scroll to top of buffer             | <kbd>⌘</kbd> + <kbd>↑</kbd>                         | <kbd>ctrl</kbd> + <kbd>↑</kbd>                      |
| `scroll_to_bottom`             | Scroll to bottom of buffer          | <kbd>⌘</kbd> + <kbd>↓</kbd>                         | <kbd>ctrl</kbd> + <kbd>↓</kbd>                      |
| `increase_font_size`           | Increase runtime font size          | <kbd>⌘</kbd> + <kbd>+</kbd>                         | <kbd>ctrl</kbd> + <kbd>+</kbd>                      |
| `decrease_font_size`           | Decrease runtime font size          | <kbd>⌘</kbd> + <kbd>-</kbd>                         | <kbd>ctrl</kbd> + <kbd>-</kbd>                      |
| `leave_buffer`                 | Leave channel or close query        | <kbd>⌘</kbd> + <kbd>shift</kbd> + <kbd>w</kbd>      | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>w</kbd>   |
| `mark_as_read`                 | Mark focused buffer as read         | <kbd>⌘</kbd> + <kbd>shift</kbd> + <kbd>m</kbd>      | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>m</kbd>   |
| `toggle_nick_list`             | Toggle nick list                    | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>m</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>m</kbd>     |
| `toggle_topic`                 | Toggle topic                        | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>t</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>t</kbd>     |
| `toggle_sidebar`               | Toggle sidebar                      | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>b</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>b</kbd>     |
| `toggle_fullscreen`            | Toggle fullscreen                   | <kbd>⌘</kbd> + <kbd>ctrl</kbd> + <kbd>f</kbd>       | <kbd>F11</kbd>                                      |
| `command_bar`                  | Toggle command bar                  | <kbd>⌘</kbd> + <kbd>k</kbd>                         | <kbd>ctrl</kbd> + <kbd>k</kbd>                      |
| `reload_configuration`         | Reload configuration file           | <kbd>⌘</kbd> + <kbd>r</kbd>                         | <kbd>ctrl</kbd> + <kbd>r</kbd>                      |
| `file_transfers`               | Toggle File Transfers Buffer        | <kbd>⌘</kbd> + <kbd>j</kbd>                         | <kbd>ctrl</kbd> + <kbd>j</kbd>                      |
| `logs`                         | Toggle Logs Buffer                  | <kbd>⌘</kbd> + <kbd>l</kbd>                         | <kbd>ctrl</kbd> + <kbd>l</kbd>                      |
| `theme_editor`                 | Toggle Theme Editor Window          | <kbd>⌘</kbd> + <kbd>t</kbd>                         | <kbd>ctrl</kbd> + <kbd>t</kbd>                      |
| `highlights`                   | Toggle Highlights Window            | <kbd>⌘</kbd> + <kbd>i</kbd>                         | <kbd>ctrl</kbd> + <kbd>i</kbd>                      |
| `quit_application`             | Quit Halloy                         | None                                                | None                                                |
| `open_config_file`             | Open settings file in system editor | <kbd>⌘</kbd> + <kbd>,</kbd>                         | <kbd>ctrl</kbd> + <kbd>,</kbd>None                  |
