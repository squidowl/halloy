# TLS Compatibility and Troubleshooting

Halloy uses [rustls](https://github.com/rustls/rustls) with its `ring` crypto
provider for TLS connections. TLS 1.2 and TLS 1.3 are enabled, using the
protocol versions, cipher suites, key exchange groups, and signature
verification algorithms provided by rustls's secure defaults. Halloy does not
add legacy cipher suites to those defaults.

The current cipher suites used by the `ring` provider are listed in the
[`rustls` documentation](https://docs.rs/rustls/latest/rustls/crypto/ring/fn.default_provider.html).
Because these defaults can change when rustls is updated, the rustls
documentation is the authoritative source rather than a list maintained here.

## Troubleshooting a TLS handshake

A server supporting TLS 1.2 or TLS 1.3 does not necessarily mean it is
compatible with Halloy. The client and server must also have a cipher suite,
signature algorithm, and key exchange group in common.

An error such as `received fatal alert: HandshakeFailure` can indicate that no
compatible parameters were found. It does not by itself identify which TLS
parameter caused the failure.

You can test a single TLS 1.2 handshake with OpenSSL:

```sh
openssl s_client -connect HOST:PORT -tls1_2 -brief
```

Or test a TLS 1.3 handshake:

```sh
openssl s_client -connect HOST:PORT -tls1_3 -brief
```

Replace `HOST` and `PORT` with the address and port used in your Halloy server
configuration. If the server uses name-based TLS configuration, connect using
its hostname and add `-servername HOST`.

A successful OpenSSL handshake only shows the single cipher suite negotiated
by that connection. To enumerate the cipher suites accepted by a server, use
Nmap:

```sh
nmap -sV --script ssl-enum-ciphers -p PORT HOST
```

For TLS 1.2, the following OpenSSL command can also check whether the server
accepts modern ECDHE cipher suites:

```sh
openssl s_client \
  -connect HOST:PORT \
  -servername HOST \
  -tls1_2 \
  -cipher 'ECDHE+AESGCM:ECDHE+CHACHA20' \
  -brief
```

If an unrestricted TLS 1.2 handshake succeeds but the ECDHE-restricted test
fails, the server may only offer obsolete cipher suites which rustls
intentionally does not support. Update the TLS configuration of the IRC server
or bouncer to offer modern cipher suites.

::: warning
[`dangerously_accept_invalid_certs`](/configuration/servers#dangerously_accept_invalid_certs)
only disables certificate validation. It does not enable unsupported protocol
versions, cipher suites, signature algorithms, or key exchange methods.
:::

## Connections protected by another transport

Disabling TLS with `use_tls = false` sends the IRC connection as plaintext.
This may be a deliberate choice when the entire connection is already carried
through a trusted encrypted transport, such as a carefully configured VPN.
Halloy cannot detect or assess the security of the surrounding network, so this
decision must be made by the user responsible for that network.
