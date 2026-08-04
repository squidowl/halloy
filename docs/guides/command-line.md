# Command Line

## Halloy

Halloy accepts a small set of command-line arguments.

| Argument | Description |
| --- | --- |
| `--version`, `-V` | Print the version and exit. Must be the first argument. |
| URL | Open an IRC or Halloy URL. See [URL Schemes](/guides/url-schemes). |

There is no `--help`, `-h`, or `-?` flag. Unknown arguments are ignored and Halloy starts normally.

Configuration and data directories are not set via command-line flags. See [Portable Mode](/guides/portable-mode).

### Examples

```bash
halloy --version
halloy ircs://irc.libera.chat/#halloy
```

## Windows installer

The Windows installer is built with [Inno Setup](https://jrsoftware.org/isinfo.php). Run the installer with `/HELP` or `/?` to list its command-line options.

Common options:

| Argument | Description |
| --- | --- |
| `/SILENT` | Hide the installer wizard, but show the installation progress window. |
| `/VERYSILENT` | Hide the installer wizard and installation progress window. |
| `/DIR="C:\Apps\Halloy"` | Set the installation directory. Use an absolute path. |
| `/TASKS="desktopicon"` | Create a desktop shortcut. |

For the full list, see the [Inno Setup command-line documentation](https://jrsoftware.org/ishelp/index.php?topic=setupcmdline).
