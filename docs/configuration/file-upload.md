# File Upload

Configuration options for file uploads via filehost. See the [File Uploads guide](/guides/filehost) for setup instructions.

## `enabled`

Enable or disable all file upload functionality.

```toml
# Type: boolean
# Values: true, false
# Default: true

[filehost]
enabled = false
```

## `button`

Show the upload button (<kbd>+</kbd>) in the message input bar.

```toml
# Type: boolean
# Values: true, false
# Default: true

[filehost]
button = false
```

## `paste`

Allow uploading files from the clipboard.

```toml
# Type: boolean
# Values: true, false
# Default: true

[filehost]
paste = false
```

## `file_drop`

Handle files dropped into the window.

```toml
# Type: boolean
# Values: true, false
# Default: false

[filehost]
file_drop = true
```
