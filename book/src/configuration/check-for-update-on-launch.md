# Check for Update on Launch

Controls whether Halloy will check the Halloy repository on launch for a new version of Halloy.  When a new version is found a dot indicator will appear on the user menu button and a menu item to open the release webpage will be added to the user menu.

This can be useful if you rather want to rely on a package manager.

- [Check for Update on Launch](#check-for-update-on-launch)
  - [Configuration](#configuration)
    - [check_for_update_on_launch](#check_for_update_on_launch)

## Configuration

### check_for_update_on_launch

> 💡 `check_for_update_on_launch` is a root key, so it must be placed before any section.

```toml
# Type: boolean
# Values: true, false
# Default: true

check_for_update_on_launch = true
```
