use mlua::{UserData, UserDataMethods};

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
