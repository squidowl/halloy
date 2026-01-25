# Tor

Tor proxy settings. Utilizes [Arti](https://arti.torproject.org/) to integrate Tor support directly into Halloy. Does not integrate into a pre-existing Tor setup.  To utilize an existing Tor daemon, use [`[proxy.socks5]`](socks5.md) instead.

It accepts no further configuration.

> ⚠️ Tor support is **not included by default**. You must build Halloy with the `tor` feature to use this proxy type. See [Optional Features](../../guides/optional-features.md) for build instructions.

> ⚠️ To preserve privacy, [previews](../preview/) are disabled when using the Tor proxy.

- [Tor](#tor)
  - [Example](#example)

## Example

```toml
[proxy.tor]
```
