# Scripts

Halloy scripts are Lua files loaded from your Halloy config directory.

- [Scripts](#scripts)
  - [Script directory](#script-directory)
  - [Running a script](#running-a-script)
    - [Manually](#manually)
    - [Automatically on startup](#automatically-on-startup)
  - [API Context (`ctx`)](#api-context-ctx)
  - [Callbacks](#callbacks)
    - [`on_start`](#on_start)
    - [`on_connect`](#on_connect)
    - [`on_notice_message`](#on_notice_message)
    - [`on_channel_message`](#on_channel_message)
    - [`on_private_message`](#on_private_message)
    - [`on_join`](#on_join)
    - [`on_part`](#on_part)
    - [`on_nick`](#on_nick)
    - [`on_mode`](#on_mode)
  - [Examples](#examples)

## Script directory

Place scripts in the `scripts` folder in your Halloy configuration directory.

```text
halloy/
├── config.toml
└── scripts/
    ├── hello.lua
    └── auto-op.lua
```

## Running a script

### Manually

Open the **Scripts** buffer and enable a script with the toggle.

### Automatically on startup

Use the config setting to load scripts when Halloy starts.
For configuration details, see [Scripts configuration](../configuration/scripts).

## API Context (`ctx`)

`ctx` is the runtime object Halloy passes into your callbacks. You use it to communicate back to Halloy.

In all callback signatures, `ctx` is the first argument.

### `ctx:log(message)`

Writes a message to Halloy logs.

- `message`: text to log.

```lua
ctx:log("script initialized")
```

### `ctx:command(raw_command)`

Sends a raw IRC command on the current server context.

- `raw_command`: raw IRC line (without leading `/`).

```lua
ctx:command("MODE #halloy +o casperstorm")
```

### `ctx:notification(name, title, body)`

Shows a script notification using your notification settings.

- `name`: key mapped to `[notifications.scripts."<name>"]`.
- `title`: notification title.
- `body`: notification text.

Configure behavior in [Notifications](../configuration/notifications/), for example:

```toml
[notifications.scripts."name"]
show_toast = true
```

```lua
ctx:notification("my_script", "Hello", "A scripted event happened")
```

## Callbacks

Scripts may define the following global Lua callbacks.
All callbacks are optional.

General callback behavior:

- A callback only runs if the function exists in your script.
- If a callback throws an error, Halloy logs it and continues running.

### `on_start`

Called once when the script is loaded.

```lua
function on_start(ctx)
end
```

Use this for one-time setup and startup logs.

### `on_connect`

Called when Halloy connects to a server.

```lua
function on_connect(ctx, server)
end
```

- `server`: server name for the current connection.

Use this for per-server initialization commands.

### `on_notice_message`

Called when a `NOTICE` is received (channel or query target).

```lua
function on_notice_message(ctx, server, target, user, text)
end
```

- `server`: server name.
- `target`: channel name (for channels) or nick (for queries).
- `user`: sender user table (`user.nick`, `user.username`, `user.hostname`).
- `text`: notice text.

### `on_channel_message`

Called when a channel `PRIVMSG` is received.

```lua
function on_channel_message(ctx, server, channel, user, text)
end
```

- `server`: server name.
- `channel`: channel target (for example `#halloy`).
- `user`: sender user table (`user.nick`, `user.username`, `user.hostname`).
- `text`: message text.

### `on_private_message`

Called when a query/private `PRIVMSG` is received.

```lua
function on_private_message(ctx, server, query, user, text)
end
```

- `query`: query target (nick).
- `user`: sender user table (`user.nick`, `user.username`, `user.hostname`).
- `text`: message text.

### `on_join`

Called when a user joins a channel.

```lua
function on_join(ctx, server, channel, user)
end
```

- `channel`: channel where the join happened.
- `user`: joining user table (`user.nick`, `user.username`, `user.hostname`).

### `on_part`

Called when a user parts a channel.

```lua
function on_part(ctx, server, channel, user)
end
```

- `channel`: channel where the part happened.
- `user`: parting user table (`user.nick`, `user.username`, `user.hostname`).

### `on_nick`

Called when a user changes nickname.

```lua
function on_nick(ctx, server, old_nick, new_nick)
end
```

- `old_nick`: previous nickname.
- `new_nick`: new nickname.

### `on_mode`

Called when a mode change is received.

```lua
function on_mode(ctx, server, target, mode, args, user)
end
```

- `target`: target receiving mode (channel or user).
- `mode`: mode string (for example `+o`).
- `args`: mode arguments array.
- `user`: source user table (`user.nick`, `user.username`, `user.hostname`), or `nil`.

## Examples

### Log Channel Messages

```lua
-- Called for every channel PRIVMSG.
function on_channel_message(ctx, server, channel, user, text)
  -- Only continue on server "libera" and channel "#foobar".
  if server ~= "libera" or channel ~= "#foobar" then
    return
  end

  -- Write the message to Halloy logs in a formatted line.
  -- Example: [libera #foobar] casper: hello
  ctx:log(string.format("[%s %s] %s: %s", server, channel, user.nick, text))
end
```

### Auto-op Multiple Nicks On Join

```lua
-- Nicks that should be auto-opped when they join.
local allow = {
  FOO = true,
  BAR = true,
  BAZ = true,
}

-- Called when a user joins a channel.
function on_join(ctx, server, channel, user)
  -- Only continue on server "libera" and channel "#foobar".
  if server ~= "libera" or channel ~= "#foobar" then
    return
  end

  -- Grant +o if the joining nick is in the allow list.
  if user and allow[user.nick] then
    ctx:command(string.format("MODE %s +o %s", channel, user.nick))
  end
end
```

### Notification Test

```lua
-- Called when a user joins a channel.
function on_join(ctx, server, channel, user)
  -- Emit a script notification whenever someone joins.
  ctx:notification(
    "script_test",
    "Join event",
    string.format("[%s %s] %s joined", server, channel, user.nick)
  )
end
```
