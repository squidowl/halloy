# Notifications

Examples

```toml
[notifications.connected]
show_toast = true

[notifications.disconnected]
sound = "dong"

[notifications.highlight]
show_toast = true
sound = "barbaz.ogg"
```

Following notifications are available:
- `connected` 
- `disconnected`
- `reconnected`
- `direct_message`
- `highlight`
- `file_transfer_request`

## `[notifications]` Section

```toml
[notifications.connected]
show_toast = true | false
sound = "<string>"
```

| Key          | Description                                                                     | Default |
| ------------ | ------------------------------------------------------------------------------- | ------- |
| `show_toast` | Notification should trigger a OS toast.                                         | `false` |
| `sound`      | Notification sound. Supports both built-in sounds[^1], and external sounds[^2]. | `""`    |

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
