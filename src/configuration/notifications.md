# Notifications

## `[notifications]` Section

```toml
[notifications.connected]
enabled = true | false
sound = "<string>"
mute = true | false

[notifications.disconnected]
enabled = true | false
sound = "<string>"
mute = true | false

[notifications.reconnected]
enabled = true | false
sound = "<string>"
mute = true | false

[notifications.highlight]
enabled = true | false
sound = "<string>"
mute = true | false

```
| Key       | Description                                           | Default                                                                                |
| --------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `enabled` | Control if notification should be enabled or not.     | `false`                                                                                |
| `mute`    | Control if the notification should have sound or not. | `false`                                                                                |
| `sound`   | The sound which plays when the notification is fired. | `"Submarine"` (macOS[^1]), `"Mail"` (Windows[^2]), `"message-new-instant"` (Linux[^3]) |

[^1]: The following sounds are available for macOS:
    * `"Basso"`
    * `"Blow"`
    * `"Bottle"`
    * `"Frog"`
    * `"Funk"`
    * `"Glass"`
    * `"Hero"`
    * `"Morse"`
    * `"Ping"`
    * `"Pop"`
    * `"Purr"`
    * `"Sosumi"`
    * `"Submarine"`
    * `"Tink" `

[^2]: The following sounds are avaiable for Windows:
    * `"Default"`
    * `"IM"`
    * `"Mail"`
    * `"Reminder"`
    * `"SMS"`

[^3]: The following sounds are avaiable for Linux:
    * `"message-new-instant"`


