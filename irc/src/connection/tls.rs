use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use tokio::fs;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::client::danger::{self, ServerCertVerifier};
use tokio_rustls::rustls::{self, pki_types};

use super::IrcStream;

pub async fn connect<'a>(
    stream: IrcStream,
    server: &str,
    accept_invalid_certs: bool,
    root_cert_path: Option<&'a PathBuf>,
    client_cert_path: Option<&'a PathBuf>,
    client_key_path: Option<&'a PathBuf>,
) -> Result<TlsStream<IrcStream>, Error> {
    let builder = if accept_invalid_certs {
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptInvalidCerts))
    } else {
        let mut roots = rustls::RootCertStore::empty();

        for cert in rustls_native_certs::load_native_certs().certs {
            let _ = roots.add(cert);
        }

        if let Some(cert_path) = root_cert_path {
            let cert_bytes = fs::read(&cert_path).await?;
            let certs = rustls_pemfile::certs(&mut Cursor::new(&cert_bytes))
                .collect::<Result<Vec<_>, _>>()?;
            roots.add_parsable_certificates(certs);
        }

        rustls::ClientConfig::builder().with_root_certificates(roots)
    };

    let client_config = if let Some(cert_path) = client_cert_path {
        let cert_bytes = Bytes::from(fs::read(&cert_path).await?);

        let key_bytes = if let Some(key_path) = client_key_path {
            Bytes::from(fs::read(&key_path).await?)
        } else {
            cert_bytes.clone()
        };

        let certs = rustls_pemfile::certs(&mut Cursor::new(&cert_bytes))
            .collect::<Result<Vec<_>, _>>()?;
        let key = rustls_pemfile::private_key(&mut Cursor::new(&key_bytes))?
            .ok_or(Error::BadPrivateKey)?;

        builder.with_client_auth_cert(certs, key)?
    } else {
        builder.with_no_client_auth()
    };

    let server_name = pki_types::ServerName::try_from(server.to_string())?;

    Ok(TlsConnector::from(Arc::new(client_config))
        .connect(server_name, stream)
        .await?)
}

#[derive(Debug)]
pub struct AcceptInvalidCerts;

impl ServerCertVerifier for AcceptInvalidCerts {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<danger::ServerCertVerified, rustls::Error> {
        Ok(danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        Ok(danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        Ok(danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("rustls error: {0}")]
    Tls(#[from] rustls::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid DNS name: {0}")]
    Dns(#[from] pki_types::InvalidDnsNameError),
    #[error("missing or invalid private key")]
    BadPrivateKey,
}
