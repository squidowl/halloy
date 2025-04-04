# Connect with ZNC

To connect with a [**ZNC**](https://wiki.znc.in/ZNC) bouncer, the configuration below can be used as a template. Simply change so it fits your credentials.

```toml
[servers.libera]
nickname = "<znc-user>/<znc-network>"
server = "znc.example.com"
password = "<your-password>"

# Depending on your ZNC setup you may need to apply these extra settings:

# Does your znc use a self-signed or expired certificate? See: 
# https://halloy.chat/configuration/servers.html#dangerously_accept_invalid_certs

# Does your znc listen on a different port? See: 
# https://halloy.chat/configuration/servers.html#port

```
