use std::path::{Path, PathBuf};
use std::sync::Arc;

use reqwest::{Client, header};
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported or invalid URI: {0}")]
    InvalidUri(String),
    #[error("refusing plaintext HTTP upload over an encrypted IRC connection")]
    InsecureTransport,
    #[error("file I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("server did not return a Location header")]
    NoLocation,
    #[error("client certificate error: {0}")]
    ClientCert(String),
}

pub enum Auth {
    Basic { username: String, password: String },
    External { cert: PathBuf, key: Option<PathBuf> },
}

/// Upload the file at `path` to `upload_url`.
///
/// Returns the public URL of the uploaded file as reported by the server's
/// `Location` header.
pub async fn upload(
    upload_url: &str,
    path: &Path,
    auth: Option<Auth>,
    irc_uses_tls: bool,
    client: Arc<Client>,
) -> Result<String, Error> {
    let base =
        Url::parse(upload_url).map_err(|e| Error::InvalidUri(e.to_string()))?;

    // Spec: clients MUST ignore tokens with an URI scheme they don't support.
    if base.scheme() != "http" && base.scheme() != "https" {
        return Err(Error::InvalidUri(format!(
            "unsupported scheme '{}'",
            base.scheme()
        )));
    }

    // Spec: clients MUST refuse unencrypted transports when IRC uses TLS.
    if irc_uses_tls && base.scheme() != "https" {
        return Err(Error::InsecureTransport);
    }

    // Spec: SASL EXTERNAL is not valid without TLS
    if matches!(auth, Some(Auth::External { .. })) && base.scheme() != "https" {
        return Err(Error::InsecureTransport);
    }

    let file_name = path.file_name().map_or_else(
        || String::from("file"),
        |n| n.to_string_lossy().into_owned(),
    );

    let bytes = tokio::fs::read(path).await?;
    let content_type = infer_mime_type(path, &bytes);

    let (external_client, auth_header) = match &auth {
        Some(Auth::External { cert, key }) => (
            Some(sasl_external_client(cert, key.as_deref()).await?),
            None,
        ),
        Some(Auth::Basic { username, password }) => {
            (None, Some(basic_auth_header(username, password)))
        }
        None => (None, None),
    };
    let upload_client = external_client.as_ref().map_or(&*client, |c| c);

    log::debug!("uploading {file_name} to {base}");

    let mut req = upload_client
        .post(base.clone())
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(bytes);

    if let Some(value) = auth_header {
        req = req.header(header::AUTHORIZATION, value);
    }

    let resp = req.send().await?;

    if resp.status().as_u16() != 201 {
        resp.error_for_status()?;
        return Err(Error::NoLocation); // satisfy compiler
    }

    let location = resp
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::NoLocation)?;

    // Spec: Resolve relative Location from host
    let file_url = base
        .join(location)
        .map_err(|e| Error::InvalidUri(e.to_string()))?;

    log::info!("file uploaded successfully: {file_url}");

    Ok(file_url.to_string())
}

/// HTTP client that presents a TLS client certificate for SASL EXTERNAL
async fn sasl_external_client(
    cert: &Path,
    key: Option<&Path>,
) -> Result<reqwest::Client, Error> {
    let cert_bytes = tokio::fs::read(cert).await?;
    let pem = if let Some(key_path) = key {
        let key_bytes = tokio::fs::read(key_path).await?;
        [cert_bytes, key_bytes].concat()
    } else {
        cert_bytes
    };

    let identity = reqwest::tls::Identity::from_pem(&pem)
        .map_err(|e: reqwest::Error| Error::ClientCert(e.to_string()))?;
    reqwest::Client::builder()
        .identity(identity)
        .build()
        .map_err(Error::Http)
}

fn basic_auth_header(username: &str, password: &str) -> String {
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(format!("{username}:{password}"));
    format!("Basic {encoded}")
}

fn infer_mime_type(path: &Path, bytes: &[u8]) -> &'static str {
    // prefer extension-based detection, fall back to byte sniffing
    if let Some(mime) = mime_guess::from_path(path).first_raw() {
        return mime;
    }
    infer::get(bytes)
        .map_or("application/octet-stream", |kind| kind.mime_type())
}
