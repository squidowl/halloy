use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use reqwest::{Client, header};
use tokio::io::AsyncReadExt as _;
use tokio_util::io::ReaderStream;
use url::Url;

use crate::config::server::Sasl;

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

#[derive(Debug)]
pub enum Auth {
    Basic { username: String, password: String },
    External { cert: PathBuf, key: Option<PathBuf> },
}

impl TryFrom<&Sasl> for Auth {
    type Error = &'static str;

    fn try_from(sasl: &Sasl) -> Result<Self, Self::Error> {
        Ok(match sasl {
            Sasl::Plain {
                username, password, ..
            } => {
                let Some(password) = password else {
                    return Err(
                        "SASL PLAIN must have password specified to be used for filehost authentication",
                    );
                };

                Self::Basic {
                    username: username.clone(),
                    password: password.clone(),
                }
            }
            Sasl::External { cert, key, .. } => Self::External {
                cert: cert.clone(),
                key: key.clone(),
            },
        })
    }
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

    let mut file = tokio::fs::File::open(path).await?;
    let file_size = file.metadata().await?.len();

    // read 36 bytes to infer file type
    let magic_len = (36usize).min(file_size as usize);
    let mut magic_buffer = vec![0u8; magic_len];
    file.read_exact(&mut magic_buffer).await?;

    let content_type = infer_mime_type(path, &magic_buffer);

    // merge magic_buffer into remaining file stream
    let stream = ReaderStream::new(Cursor::new(magic_buffer).chain(file));
    let body = reqwest::Body::wrap_stream(stream);

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

    let escaped_file_name =
        file_name.replace("\\", "\\\\").replace("\"", "\\\"");

    let mut req = upload_client
        .post(base.clone())
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{escaped_file_name}\""),
        )
        .header(header::CONTENT_LENGTH, file_size)
        .body(body);

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
