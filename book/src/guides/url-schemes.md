# URL Schemes

Halloy is able to recognize different URL schemes.

## IRC and IRCS

The IRC URL scheme is used to create a new connection to a server.  
The format is based on the [URI Syntax](https://en.wikipedia.org/wiki/Uniform_Resource_Identifier#Syntax).

## Format

```url
<scheme>://<server>:<port>/[#channel[,#channel]]
```

| Key       | Description                                                    |
| --------- | -------------------------------------------------------------- |
| `scheme`  | Can be `irc` or `ircs`. TLS is enabled if is `ircs`.           |
| `server`  | Address for the server. Eg: `irc.libera.chat`.                 |
| `port`    | Optional. Defaults to `6667` (if `irc`) or `6697` (if `ircs`). |
| `channel` | Optional. List of channels, separated by a comma.              |

### Examples

Below is a few URL examples.

- **Connect to Libera:**  
  [ircs://irc.libera.chat](ircs://irc.libera.chat)

- **Connect to Libera and join #halloy:**  
  [ircs://irc.libera.chat/#halloy](ircs://irc.libera.chat/#halloy)

- **Connect to OFTC on port 9999 and join #oftc and #asahi-dev:**  
  [ircs://irc.oftc.net:9999/#oftc,#asahi-dev](ircs://irc.oftc.net:9999/#oftc,#asahi-dev)

## Halloy

The `halloy://` scheme is used to import themes.
The syntax for that is `halloy:///theme?e=base64EncodedThemeData`.
A list of community created themes can be found [here](./configuration/themes/community.md).
