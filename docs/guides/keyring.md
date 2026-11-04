# Storing Passwords in the System Keyring

The system keyring keeps passwords out of your configuration file.

Set a keyring option to `true`:

```toml
[servers.liberachat]
password_keyring = true
```

If the entry is missing, Halloy asks for the password. Halloy then saves it in
the system keyring.

Use only one source for each password. For example, do not set both `password`
and `password_keyring`.

## Entry names

The value `true` uses an automatic entry name. You can use your own name
instead:

```toml
[servers.liberachat]
password_keyring = "my-irc-password"
```

## System support

Halloy uses these system stores:

- macOS Keychain
- Windows Credential Manager
- Secret Service on Linux and FreeBSD

On Windows, entry names are not case-sensitive.

To replace a saved password, remove it from the system store. Halloy asks for
it again when the configuration is loaded.
