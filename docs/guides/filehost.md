# File Uploads

Halloy supports file uploads via the [`soju.im/filehost`](https://soju.im/filehost) IRC extension. When a file is uploaded the resulting URL is inserted into the message input.

File uploads require a server which is capable of authenticating and accepting requests as described by `soju.im/filehost`. This can be provided by the IRC server or your bouncer.  Alternatively, any filehost server can be used for any IRC server with  [`servers.<server>.filehost.override_url`](/configuration/servers#override-url).

## Uploading a file

Uploads can be triggered by:

- Using the <kbd>+</kbd> button in the buffer
- Dragging and dropping a file into the window (if enabled with [`filehost.file_drop`](/configuration/file-upload#file-drop))
- Pasting a file into the window
- The [`/upload`](/commands#types) command

## Filehost software

This is not an exhaustive list.

If your server does not allow advertising a filehost token, or you do not control the IRC server, you can configure a URL manually via [`filehost.override_url`](/configuration/servers#override_url).

### Ergo

Ergo is not capable of hosting files, but you can point clients to a filehost server by adding the following to your `ircd.yaml`:

```yaml
server:
  additional-isupport:
    "soju.im/FILEHOST": "https://your-filehost-url/upload"
```

### soju

soju both advertises and acts as a filehost. You configure filehost [in your soju configuration](https://codeberg.org/emersion/soju/src/branch/master/doc/file-upload.md).

soju will advertise the configured URL to clients via `soju.im/FILEHOST`.

### ircv3-filehost-server

[ircv3-filehost-server](https://codeberg.org/whitequark/ircv3-filehost-server) can be used to complement an IRCd/bouncer, providing filehost functionality that the IRCd/bouncer can then advertise to clients.

### convoyeur

[convoyeur](https://codeberg.org/classabbyamp/convoyeur) can act as a proxy to external file upload services. 

## Authentication

[`filehost.credentials`](/configuration/servers#credentials) controls whether and what Halloy sends credentials with upload requests. It defaults to `"server"`, which sends the SASL credentials used to connect to the server (if they have been specified).

- **SASL PLAIN** — sends an `Authorization` header with server `username:password`
- **SASL EXTERNAL** — presents the client certificate

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
credentials = "none"
```

## Limitations

- Drag and drop is not supported on Wayland.
- Drag and drop only works in the focused buffer.
- When connecting through soju, downstream server filehosts are not passed through to child networks. Soju does not support forwarding `FILEHOST` ISUPPORT tokens from downstream servers (see [soju#374](https://codeberg.org/emersion/soju/issues/374)). You can set [`filehost.override_url`](/configuration/servers#override_url) to work around this.

## Configuration reference

- [Servers — filehost](/configuration/servers#filehost) — per-server filehost configuration
- [File Upload](/configuration/file-upload) — global filehost settings
