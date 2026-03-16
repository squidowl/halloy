# Sysinfo

Configure which system information components to display when using the `/sysinfo` command

## cpu

Show CPU information (processor brand and model)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
cpu = true
```

## memory

Show memory information

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
memory = true
```

## gpu

Show graphics card information (adapter and backend)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
gpu = true
```

## os

Show operating system information (version and kernel)

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
os = true
```

## uptime

Show system uptime information

```toml
# Type: boolean
# Values: true, false
# Default: true

[buffer.commands.sysinfo]
uptime = true
```
