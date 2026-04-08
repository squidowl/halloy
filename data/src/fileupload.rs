use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use any_ascii::any_ascii;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
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

    let mut req = upload_client
        .post(base.clone())
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, content_disposition(&file_name))
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

fn content_disposition(file_name: &str) -> String {
    let ascii: String = any_ascii(file_name)
        .chars()
        .map(|c| match c {
            '"' | '\\' | '/' => '_',
            c if c.is_ascii_control() => '_',
            c => c,
        })
        .collect();

    // rfc 8187 percent-encoded utf-8
    let utf_8 = utf8_percent_encode(file_name, NON_ALPHANUMERIC).to_string();

    format!("inline; filename=\"{ascii}\"; filename*=UTF-8''{utf_8}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_passthrough() {
        let cd = content_disposition("hello.txt");
        assert!(cd.contains("filename=\"hello.txt\""));
        assert!(cd.contains("filename*=UTF-8''hello%2Etxt"));
    }

    #[test]
    fn test_accented_latin() {
        let cd = content_disposition("café.txt");
        assert!(cd.contains("filename=\"cafe.txt\""));
        assert!(cd.contains("filename*=UTF-8''caf%C3%A9%2Etxt")); // spellchecker:disable-line
    }

    #[test]
    fn test_cjk() {
        let cd = content_disposition("中文.txt");
        // any_ascii transliterates, not underscores
        assert!(cd.contains("filename=\"ZhongWen.txt\""));
        assert!(cd.contains("filename*=UTF-8''%E4%B8%AD%E6%96%87%2Etxt"));
    }

    #[test]
    fn test_quote_substitution() {
        let cd = content_disposition("say \"hello\".txt");
        assert!(cd.contains("filename=\"say _hello_.txt\""));
        // verify the raw quote never appears inside the filename= value
        let filename_part = cd.split("filename=\"").nth(1).unwrap();
        let inside_quotes = filename_part.split('"').next().unwrap();
        assert!(!inside_quotes.contains('"'));
    }

    #[test]
    fn test_backslash_substitution() {
        let cd = content_disposition("path\\file.txt");
        assert!(cd.contains("filename=\"path_file.txt\""));
    }

    #[test]
    fn test_forward_slash_substitution() {
        let cd = content_disposition("path/file.txt");
        assert!(cd.contains("filename=\"path_file.txt\""));
    }

    #[test]
    fn test_control_chars() {
        let cd = content_disposition("bad\x01name.txt");
        assert!(cd.contains("filename=\"bad_name.txt\""));
    }
}
