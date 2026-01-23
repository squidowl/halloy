use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::fs;
use url::Url;

use super::{Preview, image};
use crate::{config, environment};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ok(Preview),
    Error,
}

pub async fn load(
    url: &Url,
    client: Arc<reqwest::Client>,
    config: &config::Preview,
) -> Option<State> {
    let path = state_path(url);

    if !path.exists() {
        return None;
    }

    let state: State =
        serde_json::from_slice(&fs::read(&path).await.ok()?).ok()?;

    // Ensure the actual image is cached
    match &state {
        State::Ok(Preview::Card(card)) => {
            if !card.image.path.exists() {
                super::fetch(card.image.url.clone(), client, config)
                    .await
                    .ok()?;
            }
        }
        State::Ok(Preview::Image(image)) => {
            if !image.path.exists() {
                super::fetch(image.url.clone(), client, config).await.ok()?;
            }
        }
        State::Error => {}
    }

    Some(state)
}

pub async fn save(url: &Url, state: State) {
    let path = state_path(url);

    if let Some(parent) = path.parent().filter(|p| !p.exists()) {
        let _ = fs::create_dir_all(parent).await;
    }

    let Ok(bytes) = serde_json::to_vec(&state) else {
        return;
    };

    let _ = fs::write(path, &bytes).await;
}

fn state_path(url: &Url) -> PathBuf {
    let hash =
        hex::encode(seahash::hash(url.as_str().as_bytes()).to_be_bytes());

    environment::cache_dir()
        .join("previews")
        .join("state")
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(&hash[4..6])
        .join(format!("{hash}.json"))
}

pub(super) fn download_path(url: &Url) -> PathBuf {
    let hash = seahash::hash(url.as_str().as_bytes());
    // Unique download path so if 2 identical URLs are downloading
    // at the same time, they don't clobber eachother
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();

    environment::cache_dir()
        .join("previews")
        .join("downloads")
        .join(format!("{hash}-{nanos}.part"))
}

pub(super) fn image_path(
    format: &image::Format,
    digest: &image::Digest,
) -> PathBuf {
    environment::cache_dir()
        .join("previews")
        .join("images")
        .join(&digest.as_ref()[..2])
        .join(&digest.as_ref()[2..4])
        .join(&digest.as_ref()[4..6])
        .join(format!(
            "{}.{}",
            digest.as_ref(),
            format.extensions_str()[0]
        ))
}
