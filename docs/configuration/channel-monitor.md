# Channel Monitor

## `exclude`

[Exclusion conditions](/configuration/conditions.md) for channels which will
not appear in Channel Monitor.

```toml
[channel_monitor]
exclude = { channels = ["#noisy-channel"] }
```

To exclude a channel only on a specific server, use a combined criterion.

```toml
[channel_monitor]
exclude = { criteria = [{ server = "libera", channel = "#noisy-channel" }] }
```

## `include`

[Inclusion conditions](/configuration/conditions.md) take precedence over
exclusion conditions. To show only selected channels, exclude everything and
then include those channels.

```toml
[channel_monitor]
exclude = "all"
include = { channels = ["#halloy", "#rust"] }
```
