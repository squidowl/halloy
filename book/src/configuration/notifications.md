# `[notifications]`

Customize and enable notifications.

**Example**

```toml
[notifications]
direct_message = { sound = "peck", show_toast = true }

[notifications.highlight]
sound = "dong"
exclude = ["NickServ", "#halloy"]
```

Following notifications are available:

| Name                    | Description                                        |
| ----------------------- | -------------------------------------------------- |
| `connected`             | Triggered when a server is connected               |
| `direct_message`        | Triggered when a direct message is received        |
| `disconnected`          | Triggered when a server disconnects                |
| `file_transfer_request` | Triggered when a file transfer request is received |
| `highlight`             | Triggered when you were highlighted in a buffer    |
| `monitored_online`      | Triggered when a user you're monitoring is online  |
| `monitored_offline`     | Triggered when a user you're monitoring is offline |
| `reconnected`           | Triggered when a server reconnects                 |


## `sound`

Notification sound.
Supports both built-in sounds, and external sound files (`mp3`, `ogg`, `flac` or `wav` placed inside the `sounds` folder within the configuration directory).

```toml
# Type: string
# Values: "dong", "peck", "ring", "squeak", "whistle", "bonk", "sing" or external sound.
# Default: not set

[notifications.<notification>]
sound = "dong"
```

## `show_toast`

Notification should trigger a OS toast.

```toml
# Type: boolean
# Values: true, false
# Default: false

[notifications.<notification>]
show_toast = true
```

## `delay`

Delay in milliseconds before triggering the next notification.

```toml
# Type: integer
# Values: any positive integer
# Default: 500

[notifications.<notification>]
delay = 250
```

## `exclude`

Exclude notifications for nicks (and/or channels in `highlight`'s case).

Only available for `direct_message`, `highlight` and `file_transfer_request`
notifications.

You can also exclude all nicks/channels by using a wildcard: `["*"]` or `["all"]`.

```toml
# Type: array of strings
# Values: array of strings
# Default: []

[notifications.<direct_message|file_transfer_request>]
exclude = ["HalloyUser1"]

[notifications.highlight]
exclude = ["HalloyUser1", "#halloy"]
```

## `include`

Include notifications for nicks (and/or channels in `highlight`'s case).

Only available for `direct_message`, `highlight` and `file_transfer_request`
notifications.

The include rule takes priority over exclude, so you can use both together.
For example, you can exclude all nicks with `["*"]` for `direct_message` and
then only include a few specific nicks to receive `direct_message` notifications
from.

```toml
# Type: array of strings
# Values: array of strings
# Default: []

[notifications.<direct_message|file_transfer_request>]
include = ["HalloyUser1"]

[notifications.highlight]
include = ["HalloyUser1", "#halloy"]
```
