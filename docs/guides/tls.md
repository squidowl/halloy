# TLS Compatibility and Troubleshooting

Halloy uses the [rustls](https://github.com/rustls/rustls) `ring` crypto
provider for TLS connections. TLS 1.2 and TLS 1.3 are enabled, with the
protocol versions, cipher suites, key exchange groups, and signature
verification algorithms provided by rustls's secure defaults. The definitive
listing of the default cipher suites can be found in the [`rustls`
documentation](https://docs.rs/rustls/latest/rustls/crypto/ring/static.DEFAULT_CIPHER_SUITES.html),
but for convenience it is reproduced here:

| TLS 1.2                                        | TLS 1.3                        |
| ---------------------------------------------- | ------------------------------ |
| TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384        | TLS13_AES_256_GCM_SHA384       |
| TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256        | TLS13_AES_128_GCM_SHA256       |
| TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256  | TLS13_CHACHA20_POLY1305_SHA256 |
| TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384          |                                |
| TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256          |                                |
| TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256    |                                |

## Troubleshooting a TLS handshake

The error `received fatal alert: HandshakeFailure` indicates a TLS handshake
could not be completed with the server. For a successful handshake, the server
*must* support TLS 1.2 or TLS 1.3, and it *must* have a cipher suite, signature
algorithm, and key exchange group in common with Halloy.

::: tip
[`dangerously_accept_invalid_certs`](/configuration/servers#dangerously_accept_invalid_certs)
only disables certificate validation. It does not enable unsupported protocol
versions, cipher suites, signature algorithms, or key exchange methods.
:::

### [OpenSSL](https://openssl-library.org/)

You can test a server's TLS 1.3 handshake with OpenSSL with the command:

```sh
echo | openssl s_client -connect HOST:PORT -tls1_3 -brief
```

where `HOST` and `PORT` should be replaced with the address and port used in
your Halloy server configuration.

Or test a TLS 1.2 handshake with:

```sh
echo | openssl s_client -connect HOST:PORT -tls1_2 -brief
```

::: tip
If connecting to a server where `HOST` is (or can be) a name, rather than an IP
address, then adding `-servername HOST` after `HOST:PORT` may be necessary. This
option sets the TLS Server Name Indication, informing the server which hostname
the client is attempting to connect to.
:::

For TLS 1.2, the following OpenSSL command can be used to check whether the
server accepts modern ECDHE cipher suites:

```sh
echo | openssl s_client \
  -connect HOST:PORT \
  -tls1_2 \
  -cipher 'ECDHE+AESGCM:ECDHE+CHACHA20' \
  -brief
```

If an unrestricted TLS 1.2 handshake succeeds but the ECDHE-restricted test
fails, the server may only offer obsolete cipher suites which rustls
intentionally does not support. If possible, it is recommended that the TLS
configuration of the IRC server or bouncer be updated to offer modern cipher
suites.

### [Nmap](https://nmap.org/)

A successful OpenSSL handshake only shows the single cipher suite negotiated by
that connection. Nmap is an alternative that can be used to enumerate the cipher
suites accepted by a server.  Using the command:

```sh
nmap -Pn --script ssl-enum-ciphers -p PORT HOST
```

Nmap will attempt to enumerate all cipher suites that the server accepts by
repeatedly initiating SSLv3/TLS connections.

## Connections protected by another transport

Halloy does not allow its accepted cipher suites to be configured. However, if
the entire connection is carried through a trusted encrypted transport, such as
a carefully configured VPN, then TLS can be disabled with `use_tls = false`
(sending the IRC connection as plaintext). It is recommended to use TLS whenever
possible, and only disable it if you understand the associated risks.
