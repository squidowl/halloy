# Connect with ZNC

::: info
Connecting with [soju](/guides/connect-with-soju) is recommended over ZNC due to the greater IRCv3 feature support it currently provides (and enables by default).
:::

To connect with a [**ZNC**](https://wiki.znc.in/ZNC) bouncer, the configuration
below can be used as a template. Simply change so it fits your credentials.

```toml
[servers.libera]
nickname ="<nickname-on-network>"
server = "znc.example.com"
username = "<znc-user>/<znc-network>"
password = "<your-password>"
```


ZNC 1.10.0 and newer allow sasl plain authentication if the [`saslplainauth`](https://wiki.znc.in/Saslplainauth) module is loaded in ZNC.

```toml
[servers.libera.sasl.plain]
username = "<znc-user>/<znc-network>"
password = "<your-password>"
```


Depending on your ZNC setup you may need to apply these extra settings:

Does your znc use a self-signed or expired certificate? See:
[`servers.<name>.dangerously_accept_invalid_certs`](/configuration/servers#dangerously_accept_invalid_certs)

Does your znc listen on a different port? See:
[`servers.<name>.port`](/configuration/servers#port)

Are you connecting with multiple clients? See:
[multiple clients](https://wiki.znc.in/Multiple_clients)

## TLS compatibility

Older ZNC or OpenSSL installations may only offer legacy cipher suites that
are not supported by Halloy's TLS library. A server offering TLS 1.2 is not
necessarily compatible if the client and server have no cipher suite in
common. See [TLS compatibility and troubleshooting](/guides/tls) for Halloy's
TLS defaults and commands that can be used to diagnose failed TLS handshakes.

ZNC administrators can configure the accepted cipher suites and protocol
versions using global configuration settings `SSLCiphers` and `SSLProtocols`.
Refer to the official [ZNC configuration
documentation](https://wiki.znc.in/Configuration) for the options supported by
the installed ZNC version. The official [ZNC hardening
documentation](https://wiki.znc.in/HardeningTest) provides further security
recommendations that are compatible with Halloy.
