# Notifications

Examples

```toml
[notifications.connected]
show_toast = true

[notifications.disconnected]
sound = { internal = "dong" }

[notifications.highlight]
show_toast = true
sound = { external = "barbaz.ogg" }
```

Following notifications are available:
- `connected` 
- `disconnected`
- `reconnected`
- `highlight`
- `file_transfer_request`

## `[notifications]` Section

```toml
[notifications.connected]
show_toast = true | false
sound = { internal | external = "<string>" }
```

| Key          | Description                                                                                                                                              | Default  |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| `show_toast` | Notification should trigger a OS toast.                                                                                                                  | `false`  |
| `sound`      | Control notification sound. Use `{ internal = "<string"> }` for a built-in sound[^1]. Use `{ external = "<string"> }` to specific an external sound[^2]. | `"none"` |

[^1]: Internal sounds: 
    - `"dong"`
    - `"peck"`
    - `"ring"`
    - `"squeak"`
    - `"whistle"`
    - `"bonk"`
    - `"sing"`
[^2]: External sounds has to be placed inside the `sounds` folder within the configuration directory. Supported formats:
    - `mp3`
    - `ogg`
    - `flac`
    - `wav`
