# Connect with ZNC

To connect with a [**ZNC**](https://wiki.znc.in/ZNC) bouncer, the configuration
below can be used as a template. Simply change so it fits your credentials.

```toml
[servers.libera]
nickname = "<znc-user>/<znc-network>"
server = "znc.example.com"
password = "<your-password>"
```


ZNC 1.10.0 and newer allow sasl plain authentication if the `saslplainauth` module is loaded in ZNC.

```toml
[servers.libera.sasl.plain]
username = "<znc-user>/<znc-network>"
password = "<your-password>"
```


Depending on your ZNC setup you may need to apply these extra settings:

Does your znc use a self-signed or expired certificate? See:
[`servers.<name>.dangerously_accept_invalid_certs`](/configuration/servers.html#dangerously_accept_invalid_certs)

Does your znc listen on a different port? See:
[`servers.<name>.port`](/configuration/servers.html#port)
