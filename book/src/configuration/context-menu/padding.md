# Padding

Adjust padding for context menu

- [Padding](#padding)
  - [Configuration](#configuration)
    - [entry](#entry)

## Configuration

### entry

Controls the padding around each entry in context menus.
The value is an array where the first value is vertical padding and the second is horizontal padding. 

```toml
# Type: array
# Values: array
# Default: [5, 5]

[context_menu.padding]
entry = [2, 5]
