# wgpu

Controls the wgpu graphics backend and GPU power preference.
These settings are applied by exporting the corresponding WGPU_* environment variables before the renderer is initialized. Environment variables always take precedence over the config file, allowing power users and packagers to override behaviour at runtime.
These settings are only applied during application startup and have no effect when the configuration is reloaded at runtime.

- [wgpu](#wgpu)
  - [Example](#example)
  - [backend](#backend)
  - [power\_pref](#power_pref)

## Example

```toml
[wgpu]
backend = "dx12"
power_pref = "high"
```

## backend

Forces a specific wgpu graphics backend. The allowed values are platform specific (see <https://github.com/gfx-rs/wgpu?tab=readme-ov-file#supported-platforms>).

```to
# Type: string
# Values: "auto", "vulkan", "metal, "dx12", "opengl"
# Default: "auto"

[wgpu]
backend = "auto"
```

Notes

- "auto" leaves backend selection to wgpu.
- This setting maps to the WGPU_BACKEND environment variable.

## power_pref

Forces a specific GPU power preference.

```to
# Type: string
# Values: "low", "high"
# Default: not set (wgpu default behaviour)

[wgpu]
power_pref = "low"
```

Notes

- "low" prefers integrated GPUs (lower power usage).
- "high" prefers dedicated GPUs (higher performance).

This setting maps to the WGPU_POWER_PREF environment variable.
