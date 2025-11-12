# User Menu

User menu in sidebar settings.

- [User Menu](#user-menu)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [show\_new\_version\_indicator](#show_new_version_indicator)


## Configuration

### enabled

Controls whether the user menu is shown in the sidebar or hidden

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.user_menu]
enabled = true
```

### show_new_version_indicator

Controls whether to show a dot indicator on the user menu button when a new version of Halloy is available.
This can be useful if you rather want to rely on a package manager.

```toml
# Type: boolean
# Values: true, false
# Default: true

[sidebar.user_menu]
show_new_version_indicator = true
```

