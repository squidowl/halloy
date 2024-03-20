# Keyboard

## `[keyboard]` Section

```toml
[keyboard]
move_up = "<string>"
move_down = "<string>"
move_left = "<string>"
move_right = "<string>"
close_buffer = "<string>"
maximize_buffer = "<string>"
restore_buffer = "<string>"
cycle_next_buffer = "<string>"
cycle_previous_buffer = "<string>"
toggle_nick_list = "<string>"
command_bar = "<string>"
```

| Key                     | Description              | Default MacOS                                       | Default Other                                       |
| ----------------------- | ------------------------ | --------------------------------------------------- | --------------------------------------------------- |
| `move_up`               | Moves focus up           | <kbd>⌥</kbd> + <kbd>↑</kbd>                         | <kbd>alt</kbd> + <kbd>↑</kbd>                       |
| `move_down`             | Moves focus down         | <kbd>⌥</kbd> + <kbd>↓</kbd>                         | <kbd>alt</kbd> + <kbd>↓</kbd>                       |
| `move_left`             | Moves focus left         | <kbd>⌥</kbd> + <kbd>←</kbd>                         | <kbd>alt</kbd> + <kbd>←</kbd>                       |
| `move_right`            | Moves focus right        | <kbd>⌥</kbd> + <kbd>→</kbd>                         | <kbd>alt</kbd> + <kbd>→</kbd>                       |
| `close_buffer`          | Close focused buffer     | <kbd>⌘</kbd> + <kbd>w</kbd>                         | <kbd>ctrl</kbd> + <kbd>w</kbd>                      |
| `maximize_buffer`       | Maximize focused buffer  | <kbd>⌘</kbd> + <kbd>↑</kbd>                         | <kbd>ctrl</kbd> + <kbd>↑</kbd>                      |
| `restore_buffer`        | Restore focused buffer   | <kbd>⌘</kbd> + <kbd>↓</kbd>                         | <kbd>ctrl</kbd> + <kbd>↓</kbd>                      |
| `cycle_next_buffer`     | Cycle to next buffer     | <kbd>ctrl</kbd> + <kbd>tab</kbd>                    | <kbd>ctrl</kbd> + <kbd>tab</kbd>                    |
| `cycle_previous_buffer` | Cycle to previous buffer | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>tab</kbd> | <kbd>ctrl</kbd> + <kbd>shift</kbd> + <kbd>tab</kbd> |
| `toggle_nick_list`      | Toggle nick list         | <kbd>⌘</kbd> + <kbd>⌥</kbd> + <kbd>m</kbd>          | <kbd>ctrl</kbd> + <kbd>alt</kbd> + <kbd>m</kbd>     |
| `command_bar`           | Toggle command bar       | <kbd>⌘</kbd> + <kbd>k</kbd>                         | <kbd>ctrl</kbd> + <kbd>k</kbd>                      |

Example for vim like movement

```toml
[keyboard]
move_up = "alt+k"
move_down = "alt+j"
move_left = "alt+h"
move_right = "alt+l"
```