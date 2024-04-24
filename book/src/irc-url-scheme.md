# IRC URL Scheme

Halloy is able to recongize IRC URL schemes for creating new connections.
The format is based on the [URI Syntax](https://en.wikipedia.org/wiki/Uniform_Resource_Identifier#Syntax).

## Format

```
<scheme>://<server>:<port>/[#channel[,#channel]]
```

| Key       | Description                                                    |
| --------- | -------------------------------------------------------------- |
| `scheme`  | Can be `irc` or `ircs`. TLS is enabled if is `ircs`.           |
| `server`  | Address for the server. Eg: `irc.libera.chat`.                 |
| `port`    | Optional. Defaults to `6667` (if `irc`) or `6697` (if `ircs`). |
| `channel` | Optional. List of channels, separated by a comma.              |


## Examples

Below is a few URL examples.

## Examples

- **Connect to Libera:**  
  [ircs://irc.libera.chat](ircs://irc.libera.chat)

- **Connect to Libera and join #halloy:**  
  [ircs://irc.libera.chat/#halloy](ircs://irc.libera.chat/#halloy)

- **Connect to OFTC on port 9999 and join #oftc and #asahi-dev:**  
  [ircs://irc.oftc.net:9999/#oftc,#asahi-dev](ircs://irc.oftc.net:9999/#oftc,#asahi-dev)

