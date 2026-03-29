use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use crate::environment;

#[derive(Debug, Default, Clone)]
pub struct KnownFilehosts {
    pub urls: HashSet<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl KnownFilehosts {
    pub fn load() -> Self {
        let path = match path() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("failed to get known filehosts path: {e}");
                return Self::default();
            }
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => Self {
                urls: contents.lines().map(str::to_owned).collect(),
            },
            Err(e) => {
                log::warn!("failed to read known filehosts: {e}");
                Self::default()
            }
        }
    }

    pub fn contains(&self, url: &str) -> bool {
        self.urls.contains(url)
    }

    pub fn insert(&mut self, url: String) {
        self.urls.insert(url);
    }

    pub async fn save(&self) -> Result<(), Error> {
        let path = path()?;
        let mut urls: Vec<&str> =
            self.urls.iter().map(String::as_str).collect();
        urls.sort();
        tokio::fs::write(path, urls.join("\n")).await?;
        Ok(())
    }
}

fn path() -> Result<PathBuf, io::Error> {
    let parent = environment::data_dir();
    if !parent.exists() {
        std::fs::create_dir_all(&parent)?;
    }
    Ok(parent.join("known_filehosts"))
}
