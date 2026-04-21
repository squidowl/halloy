use mlua::{UserData, UserDataMethods};

use crate::Server;

#[derive(Debug, Clone)]
pub enum Action {
    Command(String),
    Notification {
        name: String,
        title: String,
        body: String,
    },
}

pub struct Api {
    actions: Vec<Action>,
}

impl Api {
    pub fn new() -> Self {
        Self { actions: vec![] }
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.actions)
    }
}

impl Action {
    pub fn into_script_action(
        self,
        server: Option<&Server>,
        key: &str,
    ) -> Option<super::Action> {
        let Some(server) = server else {
            log::warn!("script {key} requested action without server context");
            return None;
        };

        Some(match self {
            Action::Command(command) => super::Action::Command {
                server: server.clone(),
                command,
            },
            Action::Notification { name, title, body } => {
                super::Action::Notification {
                    server: server.clone(),
                    name,
                    title,
                    body,
                }
            }
        })
    }
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

impl UserData for Api {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("log", |_, _this, message: String| {
            log::info!("[script] {message}");
            Ok(())
        });

        methods.add_method_mut("command", |_, this, command: String| {
            this.actions.push(Action::Command(command));
            Ok(())
        });

        methods.add_method_mut(
            "notification",
            |_, this, (script, title, body): (String, String, String)| {
                this.actions.push(Action::Notification {
                    name: script,
                    title,
                    body,
                });
                Ok(())
            },
        );
    }
}
