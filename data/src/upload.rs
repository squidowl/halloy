use std::path::Path;
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
}

pub enum Auth {
    Basic { username: String, password: String },
    Bearer(String),
}

/// Upload the file at `path` to `upload_url`.
///
/// Returns the public URL of the uploaded file as reported by the server's
/// `Location` header.
///
/// * `irc_uses_tls` — when `true`, plaintext `http://` upload URIs are
///   rejected (spec requirement: must not use unencrypted transport when the
///   IRC connection is encrypted).
pub async fn upload(
    upload_url: &str,
    path: &Path,
    auth: Option<Auth>,
    irc_uses_tls: bool,
    client: Arc<Client>,
) -> Result<String, Error> {
    let base = Url::parse(upload_url)
        .map_err(|e| Error::InvalidUri(e.to_string()))?;

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

    let file_name = path
        .file_name()
        .map_or_else(|| String::from("upload"), |n| n.to_string_lossy().into_owned());

    let bytes = tokio::fs::read(path).await?;
    let content_type = mime_for_bytes(&bytes);

    log::info!("uploading file to {base}");

    let mut req = client
        .post(base.clone())
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{file_name}\""),
        )
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(bytes);

    if let Some(auth) = auth {
        req = req.header(header::AUTHORIZATION, auth_header_value(auth));
    }

    let resp = req.send().await?;

    if resp.status().as_u16() != 201 {
        // Propagate HTTP error status.
        resp.error_for_status()?;
        // Should not be reached, but satisfies the compiler.
        return Err(Error::NoLocation);
    }

    let location = resp
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::NoLocation)?;

    // Resolve relative Location URIs against the upload base URL.
    let file_url = base
        .join(location)
        .map_err(|e| Error::InvalidUri(e.to_string()))?;

    Ok(file_url.to_string())
}

fn auth_header_value(auth: Auth) -> String {
    match auth {
        Auth::Basic { username, password } => {
            use base64::Engine as _;
            let encoded = base64::engine::general_purpose::STANDARD
                .encode(format!("{username}:{password}"));
            format!("Basic {encoded}")
        }
        Auth::Bearer(token) => format!("Bearer {token}"),
    }
}

fn mime_for_bytes(bytes: &[u8]) -> &'static str {
    infer::get(bytes)
        .map(|kind| kind.mime_type())
        .unwrap_or("application/octet-stream")
}
