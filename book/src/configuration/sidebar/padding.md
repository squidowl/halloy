# Padding

Adjust padding for sidebar

- [Padding](#padding)
  - [Configuration](#configuration)
    - [buffer](#buffer)


## Configuration

### buffer

Controls padding for buffer buttons (server, channels, queries) in the sidebar
The value is an array where the first value is vertical padding and the second is horizontal padding. 

```toml
# Type: array
# Values: array
# Default: [5, 5]

[sidebar.padding]
buffer = [2, 5]
```
