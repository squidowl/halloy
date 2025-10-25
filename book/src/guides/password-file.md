# Storing Passwords in a File

If you need to commit your configuration file to a public repository, you can keep your passwords in a separate file for security. Below is an example of using a file for nickname password for NICKSERV.


> 💡 Avoid adding extra lines in the password file, as they will be treated as part of the password.

> 💡 Windows path strings should usually be specified as literal strings (e.g. `'C:\Users\Default\'`), otherwise directory separators will need to be escaped (e.g. `"C:\\Users\\Default\\"`).

```toml
[servers.liberachat]
nickname = "foobar"
nick_password_file = "~/.config/halloy/password"
server = "irc.libera.chat"
channels = ["#halloy"]
```
