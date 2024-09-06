## `[sasl.plain]`

External SASL auth uses a PEM encoded X509 certificate. [Reference](https://libera.chat/guides/certfp).

**Example**

```toml
[servers.liberachat.sasl.plain]
cert = "/path/to/your/certificate.pem"
key = "/path/to/your/private_key.pem"
```

## `cert`

The path to PEM encoded X509 user certificate for external auth.[^1]

- **type**: string
- **values**: any string
- **default**: not set

## `key`

The path to PEM encoded PKCS#8 private key for external auth (optional).[^1]

- **type**: string
- **values**: any string
- **default**: not set

[^1]: Shell expansions (e.g. `"~/"` → `"/home/user/"`) are not supported in path strings.