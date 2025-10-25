# SASL External

External SASL auth uses a PEM encoded X509 certificate. [Reference](https://libera.chat/guides/certfp).

- [SASL External](#sasl-external)
  - [Configuration](#configuration)
    - [cert](#cert)
    - [key](#key)

## Configuration

### cert

The path to PEM encoded X509 user certificate for external auth.[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.external]
cert = "/path/to/your/certificate.pem"
```

### key

The path to PEM encoded PKCS#8 private key for external auth (optional).[^1] [^2]

```toml
# Type: string
# Values: any string
# Default: not set

[servers.<name>.sasl.external]
key = "/path/to/your/private_key.pem"
```

[^1]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
[^2]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).
