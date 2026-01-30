<!-- markdownlint-disable MD033 -->
# Notifications

Customize and enable notifications.

- [Notifications](#notifications)
  - [Example](#example)
  - [Types](#types)
  - [Built-in Sounds](#built-in-sounds)
  - [Configuration](#configuration)
    - [sound](#sound)
    - [show\_toast](#show_toast)
    - [request_attention](#request_attention)
    - [show\_content](#show_content)
    - [delay](#delay)
    - [exclude](#exclude)
    - [include](#include)

## Example

```toml
[notifications]
direct_message = { sound = "peck", show_toast = true }

[notifications.highlight]
sound = "dong"
exclude = { users = ["NickServ"], channels = ["#halloy"] }
```

## Types

Following notifications are available:

| Name                    | Description                                        | Content                           |
| ----------------------- | -------------------------------------------------- | --------------------------------- |
| `channel`               | Triggered when a message is received in a channel  | Message text                      |
| `connected`             | Triggered when a server is connected               | N/A                               |
| `direct_message`        | Triggered when a direct message is received        | Message text                      |
| `disconnected`          | Triggered when a server disconnects                | N/A                               |
| `file_transfer_request` | Triggered when a file transfer request is received | File name                         |
| `highlight`             | Triggered when you were highlighted in a buffer    | Message text                      |
| `monitored_online`      | Triggered when a user you're monitoring is online  | N/A                               |
| `monitored_offline`     | Triggered when a user you're monitoring is offline | N/A                               |
| `reconnected`           | Triggered when a server reconnects                 | N/A                               |

`channel` is an array of tables, with each entry a notification for a single
channel.  For example, the following shows a toast notification for every
message in `#halloy`:

```toml
[notifications.channel."#halloy"]
show_toast = true
```

## Built-in Sounds

The following table shows all available built-in sounds

| Sound Name | Preview                                                                          |
| ---------- | -------------------------------------------------------------------------------- |
| `bloop`    | <audio controls><source src="../../sounds/bloop.ogg" type="audio/ogg"></audio>   |
| `bonk`     | <audio controls><source src="../../sounds/bonk.ogg" type="audio/ogg"></audio>    |
| `dong`     | <audio controls><source src="../../sounds/dong.ogg" type="audio/ogg"></audio>    |
| `drop`     | <audio controls><source src="../../sounds/drop.ogg" type="audio/ogg"></audio>    |
| `peck`     | <audio controls><source src="../../sounds/peck.ogg" type="audio/ogg"></audio>    |
| `ring`     | <audio controls><source src="../../sounds/ring.ogg" type="audio/ogg"></audio>    |
| `sing`     | <audio controls><source src="../../sounds/sing.ogg" type="audio/ogg"></audio>    |
| `squeak`   | <audio controls><source src="../../sounds/squeak.ogg" type="audio/ogg"></audio>  |
| `tweep`    | <audio controls><source src="../../sounds/tweep.ogg" type="audio/ogg"></audio>   |
| `whistle`  | <audio controls><source src="../../sounds/whistle.ogg" type="audio/ogg"></audio> |
| `zone`     | <audio controls><source src="../../sounds/zone.ogg" type="audio/ogg"></audio>    |

## Configuration

### sound

Notification sound. Supports both built-in sounds, and external sound files
(`mp3`, `ogg`, `flac` or `wav` placed inside the `sounds` folder within the
configuration directory).

```toml
# Type: string
# Values: see above for built-in sounds, eg: "zone" or external sound.
# Default: not set

[notifications.<notification>]
sound = "zone"
```

### show_toast

Notification should trigger a OS toast.

```toml
# Type: boolean
# Values: true, false
# Default: false

[notifications.<notification>]
show_toast = true
```

### request_attention

Notification should request user attention for the window (aka urgency).
Triggers only when the window is not focused.

```toml
# Type: boolean
# Values: true, false
# Default: false

[notifications.<notification>]
request_attention = true
```

### show_content

Notification should show the content of the trigger (as described in the [table above](#types))).

```toml
# Type: boolean
# Values: true, false
# Default: false

[notifications.<notification>]
show_content = true
```

### delay

Delay in milliseconds before triggering the next notification.

```toml
# Type: integer
# Values: any non-negative integer
# Default: 500

[notifications.<notification>]
delay = 250
```

### exclude

[Exclusion conditions](/configuration/conditions.md) in which you won't be
notified. Inclusion conditions will take precedence over exclusion conditions.
You can also exclude all conditions by setting to `"all"` or `"*"`.

Only available for `channel`, `direct_message`, `file_transfer_request`, and
`highlight` notifications.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[notifications.<direct_message|file_transfer_request>]
exclude = { users = ["HalloyUser1"] }

[notifications.highlight]
exclude = { users = ["HalloyUser1", "#halloy"] }
```

### include

[Inclusion conditions](/configuration/conditions.md) in which you will be
notified. Notifications are enabled in all conditions unless explicitly
excluded, so this setting is only relevant when combined with the `exclude`
setting.

Only available for `channel`, `direct_message`, `file_transfer_request`, and
`highlight` notifications.

```toml
# Type: inclusion/exclusion conditions
# Values: any inclusion/exclusion conditions
# Default: not set

[notifications.<direct_message|file_transfer_request>]
include = { users = ["HalloyUser1"] }

[notifications.highlight]
include = { users = ["HalloyUser1", "#halloy"] }
```
