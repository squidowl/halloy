# Storing Passwords in a File

If you need to commit your configuration file to a public repository, you can keep your passwords in separate file(s) for security.

::: info
By default, only the first line of the file is used as the password.  If a newline needs to be included in the password, then the corresponding `*_file_first_line_only` setting should be set to `false`.  E.g. set `sasl.plain.password_file_first_line_only` to `false` if the password stored in `sasl.plain.password_file` contains a newline.
:::

::: tip
Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).
:::

## Examples

Using a file for a SASL PLAIN authentication:

```toml
[servers.liberachat]
server = "irc.libera.chat"

nickname = "foobar"
sasl.plain.username = "foobar"
sasl.plain.password_file = "~/.config/halloy/password"

channels = ["#halloy"]
```

Using a file for nickname password authentication with NickServ:

```toml
[servers.liberachat]
server = "irc.libera.chat"

nickname = "foobar"
nick_password_file = "~/.config/halloy/password"

channels = ["#halloy"]
```
