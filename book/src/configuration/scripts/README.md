# Scripts

Script configuration options.

For scripting help, see the [Scripts guide](../../guides/scripts.md).

- [Scripts](#scripts)
  - [Configuration](#configuration)
    - [autorun](#autorun)

## Configuration

### autorun

Specify the script filename(s) to load when Halloy starts.

```toml
# Type: array of strings
# Values: exact script filenames
# Default: []

[scripts]
autorun = ["hello.lua", "auto-op.lua"]
```
