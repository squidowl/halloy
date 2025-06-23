# Migrating from YAML

Halloy switched configuration file format from YAML to TOML ([PR-278](https://github.com/squidowl/halloy/pull/278))
This page will help you migrate your old `config.yaml` to a new `config.toml` file.

The basic structure of a TOML file consists of key-value pairs, where keys are strings. There are no nested indentations like YAML, which makes it easier to read and write. Consider the following old YAML config with of two servers in Halloy:

```yaml
servers:
  libera:
    nickname: foobar
    server: irc.libera.chat
  quakenet:
    nickname: barbaz
    server: underworld2.no.quakenet.org
    port: 6667
    use_tls: true
```

This now looks the following in TOML

```toml
[servers.libera]
nickname = "foobar"
server = "irc.libera.chat"

[servers.quakenet]
nickname = "barbaz"
server = "underworld2.no.quakenet.org"
port = 6667
use_tls = true
```

> 💡 You can convert YAML to TOML using a converter tool like [this one](https://transform.tools/yaml-to-toml). Just note that a few keys and values have be renamed during the conversion process.

To migrate, and ensure everything is working, make sure to read through the [Configuration](../configuration) section of this book. Here, every configuration option is documented using TOML.
