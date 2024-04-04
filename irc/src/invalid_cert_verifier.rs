use std::sync::Arc;

use tokio_rustls::rustls::{
    self,
    client::{
        danger::{self, ServerCertVerifier},
        WebPkiServerVerifier,
    },
    RootCertStore,
};

#[derive(Debug)]
pub(crate) struct InvalidServerCertVerifier {
    verifier: Arc<WebPkiServerVerifier>,
}

impl InvalidServerCertVerifier {
    pub fn new(roots: impl Into<Arc<RootCertStore>>) -> InvalidServerCertVerifier {
        Self {
            verifier: WebPkiServerVerifier::builder(roots.into()).build().unwrap(),
        }
    }
}

impl ServerCertVerifier for InvalidServerCertVerifier {
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
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        self.verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        self.verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.verifier.supported_verify_schemes()
    }
}
