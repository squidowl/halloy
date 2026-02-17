use crate::config;
use crate::environment::VERSION;

const LATEST_REMOTE_RELEASE_URL: &str =
    "https://api.github.com/repos/squidowl/halloy/releases/latest";

#[derive(Debug, Clone)]
pub struct Version {
    pub current: String,
    pub remote: Option<String>,
}

impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}

impl Version {
    pub fn new() -> Version {
        let current = VERSION.to_owned();

        Version {
            current,
            remote: None,
        }
    }

    pub fn is_old(&self) -> bool {
        match &self.remote {
            Some(remote) => &self.current != remote,
            None => false,
        }
    }
}

pub async fn latest_remote_version(
    proxy: Option<config::Proxy>,
) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }

    let client = if let Some(proxy) = proxy {
        // If the proxy fails to build it should be logged when the
        // preview client is created, we can handle it silently here.
        config::proxy::build_client(&proxy).ok()?
    } else {
        reqwest::Client::builder()
            .user_agent("halloy")
            .build()
            .ok()?
    };

    let response = client
        .get(LATEST_REMOTE_RELEASE_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await
        .ok()?;

    response
        .json::<Release>()
        .await
        .ok()
        .map(|release| release.tag_name)
}
