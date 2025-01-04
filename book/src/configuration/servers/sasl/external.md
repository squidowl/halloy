## `[sasl.external]`

External SASL auth uses a PEM encoded X509 certificate. [Reference](https://libera.chat/guides/certfp).

**Example**

```toml
[servers.liberachat.sasl.external]
cert = "/path/to/your/certificate.pem"
key = "/path/to/your/private_key.pem"
```

## `cert`

The path to PEM encoded X509 user certificate for external auth.[^1] [^2]

- **type**: string
- **values**: any string
- **default**: not set

## `key`

The path to PEM encoded PKCS#8 private key for external auth (optional).[^1] [^2]

- **type**: string
- **values**: any string
- **default**: not set

[^1]: Shell expansions (e.g. `"~/"` → `"/home/user/"`) are not supported in path strings.
[^2]: Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
