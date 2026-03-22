# Filehost

Halloy supports file uploads via the [`draft/FILEHOST`](https://github.com/progval/ircv3-specifications/blob/filehost/extensions/filehost.md) IRC extension and [`soju.im/filehost`](https://soju.im/filehost). When a file is uploaded, the resulting URL is inserted into the message input.

Uploads can be triggered by:

- Using the <kbd>+</kbd> button in the buffer
- Dragging and dropping a file into the window
- Pasting a file into the window
- The [`/upload`](/commands#upload) command

::: info
Filehost requires the server to advertise support for it according to the spec. Alternatively, [`servers.<name>.filehost.override_url`](/configuration/servers#override_url) can be set to use any filehost.
:::

## Authentication

Set [`filehost.send_credentials`](/configuration/servers#send_credentials) to `true` and Halloy will send your server credentials as HTTP Basic Auth with uploads.

::: warning
Only enable `send_credentials` if you trust the filehost server. Your username and password will be sent with every upload.
:::

```toml
[servers.<name>]
server = "irc.example.com"
nickname = "you"

# credentials that will be sent to filehost
[servers.<name>.sasl.plain]
username = "you"
password = "hunter2"

[servers.<name>.filehost]
send_credentials = true
```

## Server support

A server can set an `ISUPPORT` token to provide a filehost.

### Ergo

Ergo supports filehost natively via `additional-isupport`. Add the following to your `ircd.yaml`:

```yaml
server:
  additional-isupport:
    "draft/FILEHOST": "https://your-filehost-url/upload"
```

### soju

soju supports filehost via its own `soju.im/FILEHOST` token. Configure filehost [in your soju configuration](https://codeberg.org/emersion/soju/src/branch/master/doc/file-upload.md).

soju will advertise the configured URL to clients via `soju.im/FILEHOST`. This is then passed down to your networks via bouncer-networks.

::: info
soju requires you to [authenticate](#Authentication) requests.
:::

### convoyeur

[convoyeur](https://codeberg.org/classabbyamp/convoyeur) can act as a proxy to external file upload services. 

### Other servers

If your server does not advertise a filehost token, you can configure a URL manually via [`filehost.override_url`](/configuration/servers#override_url).

## Configuration reference

See [Servers -- filehost](/configuration/servers#filehost) for the full list of configuration options.
