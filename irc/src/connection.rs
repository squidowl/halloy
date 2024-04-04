use std::io::Cursor;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::pki_types::{self, CertificateDer, PrivateKeyDer};
use tokio_rustls::{client::TlsStream, rustls, TlsConnector};
use tokio_util::codec;
use tokio_util::codec::Framed;

use crate::invalid_cert_verifier::InvalidServerCertVerifier;

pub enum Connection<Codec> {
    Tls(Framed<TlsStream<TcpStream>, Codec>),
    Unsecured(Framed<TcpStream, Codec>),
}

#[derive(Debug, Clone)]
pub enum Security<'a> {
    Unsecured,
    Secured {
        accept_invalid_certs: bool,
        root_cert_path: Option<&'a PathBuf>,
        client_cert_path: Option<&'a PathBuf>,
        client_key_path: Option<&'a PathBuf>,
    },
}

#[derive(Debug, Clone)]
pub struct Config<'a> {
    pub server: &'a str,
    pub port: u16,
    pub security: Security<'a>,
}

impl<Codec> Connection<Codec> {
    pub async fn new(config: Config<'_>, codec: Codec) -> Result<Self, Error> {
        let tcp = TcpStream::connect((config.server, config.port)).await?;

        if let Security::Secured {
            accept_invalid_certs,
            root_cert_path,
            client_cert_path,
            client_key_path,
        } = config.security
        {
            let mut roots = rustls::RootCertStore::empty();
            for cert in
                rustls_native_certs::load_native_certs().expect("could not load platform certs")
            {
                roots.add(cert).unwrap();
            }

            if let Some(root_cert_path) = root_cert_path {
                roots.add_parsable_certificates(read_certs_from_path(root_cert_path).await?);
            }

            let builder = rustls::ClientConfig::builder();
            let builder = if accept_invalid_certs {
                builder
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(InvalidServerCertVerifier {}))
            } else {
                builder.with_root_certificates(roots)
            };

            let client_config = if let (None, None) = (client_cert_path, client_key_path) {
                builder.with_no_client_auth()
            } else {
                let (certs, key) = match (client_cert_path, client_key_path) {
                    (Some(cert_path), None) => read_certs_and_key_from_path(cert_path).await?,
                    (Some(cert_path), Some(key_path)) if cert_path == key_path => {
                        read_certs_and_key_from_path(cert_path).await?
                    }
                    (Some(cert_path), Some(key_path)) => (
                        read_certs_from_path(cert_path).await?,
                        read_key_from_path(key_path).await?,
                    ),
                    (None, Some(_)) => {
                        return Err(Error::ClientCertificate(
                            CertificateError::BadCertificateFile,
                        ))
                    }
                    (None, None) => unreachable!(),
                };
                builder.with_client_auth_cert(certs, key)?
            };

            let server_name = pki_types::ServerName::try_from(config.server.to_owned())
                .expect("invalid server name");
            let tls = TlsConnector::from(Arc::new(client_config));
            let tls = tls.connect(server_name, tcp).await?;

            Ok(Self::Tls(Framed::new(tls, codec)))
        } else {
            Ok(Self::Unsecured(Framed::new(tcp, codec)))
        }
    }

    /// Binds a listener and returns a single connection
    /// once accepted. Useful for DCC flow.
    pub async fn listen_and_accept(
        address: IpAddr,
        port: u16,
        security: Security<'_>,
        codec: Codec,
    ) -> Result<Self, Error> {
        let listener = TcpListener::bind((address, port)).await?;

        let (tcp, _remote) = listener.accept().await?;

        match security {
            Security::Unsecured => Ok(Self::Unsecured(Framed::new(tcp, codec))),
            Security::Secured { .. } => {
                todo!();
            }
        }
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        match self {
            Connection::Tls(framed) => {
                framed.into_inner().shutdown().await?;
            }
            Connection::Unsecured(framed) => {
                framed.into_inner().shutdown().await?;
            }
        }
        Ok(())
    }
}

async fn read_certs_and_key_from_path<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Error> {
    let pem_bytes = fs::read(path).await?;
    let mut pem_reader = Cursor::new(pem_bytes);

    let certs = rustls_pemfile::certs(&mut pem_reader)
        .map(Result::unwrap)
        .collect();

    pem_reader.set_position(0);

    let key = rustls_pemfile::private_key(&mut pem_reader)?
        .ok_or_else(|| Error::ClientCertificate(CertificateError::BadPrivateKey))?;

    Ok((certs, key))
}

async fn read_certs_from_path<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<CertificateDer<'static>>, Error> {
    let pem_bytes = fs::read(path).await?;
    let mut pem_reader = Cursor::new(pem_bytes);

    let certs = rustls_pemfile::certs(&mut pem_reader)
        .map(Result::unwrap)
        .collect();

    Ok(certs)
}

async fn read_key_from_path<P: AsRef<Path>>(path: P) -> Result<PrivateKeyDer<'static>, Error> {
    let pem_bytes = fs::read(path).await?;
    let mut pem_reader = Cursor::new(pem_bytes);

    let key = rustls_pemfile::private_key(&mut pem_reader)?
        .ok_or_else(|| Error::ClientCertificate(CertificateError::BadPrivateKey))?;

    Ok(key)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("client certificate error: {0}")]
    ClientCertificate(CertificateError),
}

#[derive(Debug, thiserror::Error)]
pub enum CertificateError {
    #[error("missing or invalid private key")]
    BadPrivateKey,
    #[error("missing or invalid certificate file")]
    BadCertificateFile,
}

macro_rules! delegate {
    ($e:expr, $($t:tt)*) => {
        match $e {
            $crate::connection::Connection::Tls(framed) => framed.$($t)*,
            $crate::connection::Connection::Unsecured(framed) => framed.$($t)*,
        }
    };
}

impl<Codec> Stream for Connection<Codec>
where
    Codec: codec::Decoder,
{
    type Item = Result<Codec::Item, Codec::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        delegate!(self.get_mut(), poll_next_unpin(cx))
    }
}

impl<Item, Codec> Sink<Item> for Connection<Codec>
where
    Codec: codec::Encoder<Item>,
{
    type Error = Codec::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_ready_unpin(cx))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        delegate!(self.get_mut(), start_send_unpin(item))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_flush_unpin(cx))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        delegate!(self.get_mut(), poll_close_unpin(cx))
    }
}
