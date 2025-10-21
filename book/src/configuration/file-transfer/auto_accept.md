# Auto Accept

Configuration for automatically accepting incoming file transfers.

- [Auto Accept](#auto-accept)
  - [Configuration](#configuration)
    - [enabled](#enabled)
    - [nicks](#nicks)
    - [masks](#masks)


## Configuration

### enabled

If true, automatically accept incoming file transfers. Requires `save_directory` to be set.

```toml
# Type: boolean
# Values: true, false
# Default: false

[file_transfer.auto_accept]
enabled = false
```

### nicks

If true, automatically accept incoming file transfers from these nicks.
Note `auto_accept` has to be enabled.

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[file_transfer.auto_accept]
nicks = ["nick1", "nick2"]
```

### masks

If true, automatically accept incoming file transfers from these nicks. Matches are made against the full nickname (i.e. nickname, username, and hostname in the format `nickname!username@hostname`). Note `auto_accept` has to be enabled.

> 💡 Use toml multi-line literal strings `'''\bfoo'd\b'''` when writing a regex. This > allows you to write write the regex without
> escaping. You can also use a literal string `'\bfoo\b'`, but then you can't use `'` inside the string.
>
> Without literal strings, you'd have to write the above as `"\\bfoo'd\\b"`

```toml
# Type: array of strings
# Values: array of any strings
# Default: []

[file_transfer.auto_accept]
masks = [
    '''nick!ident@example\.com''',
    '''.*@foobar\.com'''
]
```

[^1]: Relative paths are prefixed with the config directory (i.e. if you have your config.toml in `/home/me/.config/halloy/config.toml`, path `.passwd/libera` will be converted to `/home/me/.config/halloy/.passwd/libera`).
