# File Uploads

Halloy supports file uploads via the [`draft/FILEHOST`](https://github.com/progval/ircv3-specifications/blob/filehost/extensions/filehost.md) and [`soju.im/filehost`](https://soju.im/filehost) IRC extensions. When a file is uploaded, the resulting URL is inserted into the message input.

Uploads can be triggered by:

- Using the <kbd>+</kbd> button in the buffer
- Dragging and dropping a file into the window
- Pasting a file into the window
- The [`/upload`](/commands#types) command

::: info
Filehost requires your server to advertise support for it according to the spec. Alternatively, [`filehost.override_url`](/configuration/servers#override_url) can be set to override with any filehost.
:::

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
soju requires you to [authenticate](#Authentication) your upload requests
:::

### convoyeur

[convoyeur](https://codeberg.org/classabbyamp/convoyeur) can act as a proxy to external file upload services. 

### Others

If your server does not allow advertising a filehost token, or you do not control your server, you can configure a URL manually via [`filehost.override_url`](/configuration/servers#override_url).

## Authentication

Set [`filehost.send_credentials`](/configuration/servers#send_credentials) to `true` and Halloy will attach your server credentials when uploading.

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

## Limitations

- Drag and drop is not supported on Wayland due to an upstream [iced limitation](https://github.com/iced-rs/iced/issues/2538).
- Drag and drop only works into the currently active buffer.
- When connecting through soju, downstream server filehosts are ignored. Soju does not currently support passing through `draft/FILEHOST` (see [soju#374](https://codeberg.org/emersion/soju/issues/374)). You can set [`filehost.override_url`](/configuration/servers#override_url) to work around this.

## Configuration reference

See [Servers — filehost](/configuration/servers#filehost) for the full list of configuration options.
