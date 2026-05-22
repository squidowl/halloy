# Runtime

Runtime graphics settings.

## `backend`

Select the graphics backend.

```toml
# Type: String
# Values: "best", "hardware", "software"
# Default: "best"

[runtime]
backend = "best"
```

`"hardware"` uses your GPU, while `"software"` is quite slower than
hardware-based backends, but more compatible.

## `vsync`

Whether frames synchronizes with your display refresh rate.

```toml
# Type: Boolean
# Values: true, false
# Default: true

[runtime]
vsync = true
```

## `antialiasing`

Whether to enable antialiasing renderer for primitives.

```toml
# Type: Boolean
# Values: true, false
# Default: false

[runtime]
antialiasing = false
```
