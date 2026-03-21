# Exec Command

The `/exec` command runs a local shell command on your machine and sends the first non-empty line of stdout to the current buffer.

::: warning
Enable `/exec` only if you trust the commands you plan to run. See [buffer command configuration](../configuration/buffer#exec).
:::

Examples for Unix-like systems:

```text
/exec printf '/me is on %s using %s' "$(hostname)" "$(uname -srm)"
```

Example random roll:

```text
/exec printf '/me rolls %s (1-6)' "$((RANDOM % 6 + 1))"
```

Since the output is sent back into the input buffer, starting the line with `/me` or another IRC command can be useful.

`/exec` also works well together with [aliases](../configuration/buffer#aliases). For commands you use often, an alias can save you from retyping the full shell command each time.
