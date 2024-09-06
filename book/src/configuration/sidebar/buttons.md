# `[sidebar.buttons]`

Buttons at the bottom of the sidebar. 

**Example**

```toml
[sidebar.buttons]
file_transfer = false
command_bar = true
reload_config = true
```

## `file_transfer`

File transfer button in sidebar which opens file transfer buffer.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `command_bar`

Command bar button in sidebar which opens the command bar.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`

## `reload_config`

Reload config button in sidebar which reloads the configuration file.

- **type**: boolean
- **values**: `true`, `false`
- **default**: `true`