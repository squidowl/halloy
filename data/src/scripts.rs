use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use mlua::Lua;

use crate::{Config, Server};

mod api;
mod callback;

pub use self::api::Api;
pub use self::callback::{
    on_channel_message, on_connect, on_join, on_mode, on_nick,
    on_notice_message, on_part, on_private_message, on_start,
};

#[derive(Debug, Clone)]
pub enum Action {
    Command {
        server: Server,
        command: String,
    },
    Notification {
        server: Server,
        name: String,
        title: String,
        body: String,
    },
}

pub struct Manager {
    scripts: HashMap<String, Script>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
        }
    }

    pub fn add(&mut self, scripts: Vec<Script>) {
        for script in scripts {
            self.scripts.entry(script.name.clone()).or_insert(script);
        }
    }

    pub fn on_start_callback(&mut self, name: &str) {
        let Some(script) = self.scripts.get(name) else {
            return;
        };

        callback::on_start(script);
    }

    pub fn load(&mut self, name: &str) -> bool {
        let Some(script) = self.scripts.get_mut(name) else {
            log::warn!("script not found: {name}");
            return false;
        };

        script.load()
    }

    pub fn is_loaded(&self, name: &str) -> Option<bool> {
        self.scripts.get(name).map(Script::is_loaded)
    }

    pub fn scripts(&self) -> impl Iterator<Item = &Script> {
        self.scripts.values()
    }

    pub fn unload(&mut self, name: &str) {
        let Some(script) = self.scripts.get_mut(name) else {
            return;
        };

        script.unload();
    }

    pub fn refresh(&mut self, scripts: Vec<Script>, autorun: &[String]) {
        let scripts_to_enable: HashSet<_> = self
            .scripts
            .iter()
            .filter_map(|(name, script)| {
                script.is_loaded().then_some(name.clone())
            })
            .chain(autorun.iter().cloned())
            .collect();

        let mut refreshed_scripts = HashMap::new();
        for script in scripts {
            refreshed_scripts.insert(script.name.clone(), script);
        }

        self.scripts = refreshed_scripts;

        for name in scripts_to_enable {
            if self.load(&name) {
                self.on_start_callback(&name);
            }
        }
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Script {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    lua: Option<Lua>,
}

impl std::fmt::Debug for Script {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Script")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("source", &self.source)
            .field("loaded", &self.is_loaded())
            .finish()
    }
}

impl Script {
    pub fn new(name: String, path: PathBuf, source: String) -> Self {
        Self {
            name,
            path,
            source,
            lua: None,
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.lua.is_some()
    }

    pub fn unload(&mut self) {
        self.lua = None;
    }

    pub fn load(&mut self) -> bool {
        if self.is_loaded() {
            return false;
        }

        let lua = Lua::new();

        if let Err(error) = lua
            .load(&self.source)
            .set_name(self.path.to_string_lossy().as_ref())
            .exec()
        {
            log::error!("failed to load script {:?}: {error}", self.path);
            return false;
        }

        self.lua = Some(lua);

        true
    }

    pub fn lua(&self) -> Option<&Lua> {
        self.lua.as_ref()
    }
}

pub async fn parse() -> Vec<Script> {
    let scripts_dir = Config::scripts_dir();
    let mut scripts = parse_directory(&scripts_dir).await;

    scripts.sort_by(|a, b| a.path.cmp(&b.path));

    scripts
}

async fn parse_directory(path: &PathBuf) -> Vec<Script> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(error) => {
            log::error!("failed to read scripts dir at {path:?}: {error}");
            return vec![];
        }
    };

    let mut scripts = vec![];

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        let is_lua = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lua"));

        if !is_lua {
            continue;
        }

        let source = match tokio::fs::read_to_string(&path).await {
            Ok(source) => source,
            Err(error) => {
                log::error!("failed to read script {path:?}: {error}");
                continue;
            }
        };

        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            log::error!(
                "failed to derive script name from path {path:?}; skipping"
            );
            continue;
        };

        scripts.push(Script::new(name, path, source));
    }

    scripts
}
