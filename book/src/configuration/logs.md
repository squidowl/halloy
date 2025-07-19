# `[logs]`

Customize log buffer

# `file_level`

The least urgent (most verbose) log level to record to the log file.
E.g. a `file_level` setting of `"debug"` will record all `ERROR`, `WARN`, `INFO`, and `DEBUG` messages to the log file.
The log file is overwritten on each launch (i.e. contains log messages for the last session only).  It can be accessed at:

* Windows: `%AppData%\Roaming\halloy\halloy.log`
* Mac: `~/Library/Application Support/halloy/halloy.log` or `$HOME/.local/share/halloy/halloy.log`
* Linux: `$XDG_DATA_HOME/halloy/halloy.log` or `$HOME/.local/share/halloy/halloy.log`

> ⚠️  Changes to file_level require an application restart to take effect.

```toml
# Type: string
# Values: "off", "error", "warn", "info", "debug", "trace"
# Default: "debug"

[logs]
file_level = "debug"
```

# `pane_level`

The least urgent (most verbose) log level to record to the Logs pane.
E.g. a `pane_level` setting of `"info"` will record all `ERROR`, `WARN`, and `INFO` messages to the Logs pane.
Log messages that are not recorded to the Logs pane may still be found in the log file.

```toml
# Type: string
# Values: "off", "error", "warn", "info", "debug", "trace"
# Default: "info"

[logs]
pane_level = "info"
```

