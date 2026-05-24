# Runtime

Runtime graphics settings.

## `backend`

Select the graphics backend.

```toml
# Type: string or object
# Values: "best", "hardware", "software"
# Default: "best"

[runtime]
backend = "best"
```

`"hardware"` uses your GPU, while `"software"` is quite slower than
hardware-based backends, but more compatible.

The hardware backend can be configured to request a specific graphics API:

```toml
# Type: object
# Values: "best", "vulkan", "metal", "directx12", "opengl", "webgpu"

[runtime]
backend = { hardware = "best" }
```

## `vsync`

Whether frames synchronizes with your display refresh rate.

```toml
# Type: boolean
# Values: true, false
# Default: true

[runtime]
vsync = true
```

## `antialiasing`

Whether to enable antialiasing renderer for primitives.

```toml
# Type: boolean
# Values: true, false
# Default: false

[runtime]
antialiasing = false
```
