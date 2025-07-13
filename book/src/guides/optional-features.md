# Optional Features

Halloy supports optional features that can be enabled during compilation to add additional functionality. These features are not included by default to keep the binary size small and compilation fast.

## Building with features

To build Halloy with specific features, use the `--features` flag:

```bash
# Build with a feature
cargo build --features tor

# Build release with features
cargo build --release --features tor
```

## Available features

### `tor`

Enables Tor network support for anonymous IRC connections. 
Not enabled by default. 

See [Proxy Configuration](../configuration/proxy.md#proxytor) for usage details.