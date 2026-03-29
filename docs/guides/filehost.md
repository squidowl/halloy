# File Uploads

Halloy supports file uploads via the `draft/FILEHOST` and [`soju.im/filehost`](https://soju.im/filehost) IRC extensions. When a file is uploaded, the resulting URL is inserted into the message input.

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

soju will advertise the configured URL to clients via `soju.im/FILEHOST`. For networks provided by bouncer, Halloy will use the bouncer's filehost.

::: info
soju requires you to [authenticate](#Authentication) your upload requests
:::

### convoyeur

[convoyeur](https://codeberg.org/classabbyamp/convoyeur) can act as a proxy to external file upload services. 

### Others

If your server does not allow advertising a filehost token, or you do not control the IRC server, you can configure a URL manually via [`filehost.override_url`](/configuration/servers#override_url).

## Authentication

[`filehost.send_credentials`](/configuration/servers#send_credentials) controls whether Halloy sends credentials with upload requests. It defaults to `true`.

- **SASL PLAIN** — sends an `Authorization` header with server `username:password`
- **SASL EXTERNAL** — presents the client certificate

::: warning
Only send credentials to filehosts you trust.
:::

```toml
[servers.<name>]
server = "irc.example.com"
nickname = "you"

# credentials that will be sent to filehost
[servers.<name>.sasl.plain]
username = "you"
password = "hunter2"

# alternatively, disable sending credentials when uploading
[servers.<name>.filehost]
send_credentials = false
```

## Limitations

- Drag and drop is not supported on Wayland.
- Drag and drop only works in the active buffer.
- When connecting through soju, downstream server filehosts are not passed through to child networks. Soju does not support forwarding `FILEHOST` ISUPPORT tokens from downstream servers (see [soju#374](https://codeberg.org/emersion/soju/issues/374)). You can set [`filehost.override_url`](/configuration/servers#override_url) to work around this.

## Configuration reference

- [Servers — filehost](/configuration/servers#filehost) — per-server filehost configuration
- [File Upload](/configuration/file-upload) — global filehost settings
