# [notifications]

Customize and enable notifications.

**Example**

```toml
[notifications]
highlight = { sound = "dong" }
direct_message = { sound = "peck", show_toast = true }
```

Following notifications are available:

| Name                    | Description                                        |
| ----------------------- | -------------------------------------------------- |
| `connected`             | Triggered when a server is connected               |
| `direct_message`        | Triggered when a direct message is received        |
| `disconnected`          | Triggered when a server disconnects                |
| `file_transfer_request` | Triggered when a file transfer request is received |
| `highlight`             | Triggered when you were highlighted in a buffer    |
| `reconnected`           | Triggered when a server reconnects                 |


## `sound`

Notification sound.
Supports both built-in sounds, and external sound files (`mp3`, `ogg`, `flac` or `wav` placed inside the `sounds` folder within the configuration directory).

- **type**: string
- **values**: `"dong"`, `"peck"`, `"ring"`, `"squeak"`, `"whistle"`, `"bonk"`, `"sing"` or external sound.
- **default**: not set


## `show_toast`

Notification should trigger a OS toast.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `false`