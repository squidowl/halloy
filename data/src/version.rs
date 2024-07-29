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

pub async fn latest_remote_version() -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
    }

    let client = reqwest::Client::builder()
        .user_agent("halloy")
        .build()
        .ok()?;

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
