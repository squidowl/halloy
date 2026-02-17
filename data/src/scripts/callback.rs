use mlua::{AnyUserData, Function, Lua, Table};

use super::api::Action as ApiAction;
use super::{Action, Api, Script};
use crate::{Server, User};

pub fn on_start(script: &Script) {
    let _ = dispatch(
        std::iter::once(script),
        None,
        "on_start",
        |_, callback, context| callback.call::<()>((context.clone(),)),
    );
}

pub fn on_connect<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
) -> Vec<Action> {
    let server_name = server.to_string();

    dispatch(
        scripts,
        Some(server),
        "on_connect",
        |_, callback, context| {
            callback.call::<()>((context.clone(), server_name.as_str()))
        },
    )
}

pub fn on_join<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    channel: &str,
    user: Option<&User>,
) -> Vec<Action> {
    let server_name = server.to_string();
    let Some(user) = user else {
        return vec![];
    };

    dispatch(
        scripts,
        Some(server),
        "on_join",
        |lua, callback, context| {
            let Some(user) = create_user(lua, user) else {
                return Ok(());
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                channel,
                user,
            ))
        },
    )
}

pub fn on_part<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    channel: &str,
    user: Option<&User>,
) -> Vec<Action> {
    let server_name = server.to_string();
    let Some(user) = user else {
        return vec![];
    };

    dispatch(
        scripts,
        Some(server),
        "on_part",
        |lua, callback, context| {
            let Some(user) = create_user(lua, user) else {
                return Ok(());
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                channel,
                user,
            ))
        },
    )
}

pub fn on_nick<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    old_nick: &str,
    new_nick: &str,
) -> Vec<Action> {
    let server_name = server.to_string();

    dispatch(scripts, Some(server), "on_nick", |_, callback, context| {
        callback.call::<()>((
            context.clone(),
            server_name.as_str(),
            old_nick,
            new_nick,
        ))
    })
}

pub fn on_channel_message<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    channel: &str,
    user: Option<&User>,
    text: &str,
) -> Vec<Action> {
    let server_name = server.to_string();
    let Some(user) = user else {
        return vec![];
    };

    dispatch(
        scripts,
        Some(server),
        "on_channel_message",
        |lua, callback, context| {
            let Some(user) = create_user(lua, user) else {
                return Ok(());
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                channel,
                user,
                text,
            ))
        },
    )
}

pub fn on_private_message<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    query: &str,
    user: Option<&User>,
    text: &str,
) -> Vec<Action> {
    let server_name = server.to_string();
    let Some(user) = user else {
        return vec![];
    };

    dispatch(
        scripts,
        Some(server),
        "on_private_message",
        |lua, callback, context| {
            let Some(user) = create_user(lua, user) else {
                return Ok(());
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                query,
                user,
                text,
            ))
        },
    )
}

pub fn on_notice_message<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    target: &str,
    user: Option<&User>,
    text: &str,
) -> Vec<Action> {
    let server_name = server.to_string();
    let Some(user) = user else {
        return vec![];
    };

    dispatch(
        scripts,
        Some(server),
        "on_notice_message",
        |lua, callback, context| {
            let Some(user) = create_user(lua, user) else {
                return Ok(());
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                target,
                user,
                text,
            ))
        },
    )
}

pub fn on_mode<'a>(
    scripts: impl Iterator<Item = &'a Script>,
    server: &Server,
    target: &str,
    mode: &str,
    args: &[String],
    user: Option<&User>,
) -> Vec<Action> {
    let server_name = server.to_string();

    dispatch(
        scripts,
        Some(server),
        "on_mode",
        |lua, callback, context| {
            let user = match user {
                Some(user) => create_user(lua, user),
                None => None,
            };

            callback.call::<()>((
                context.clone(),
                server_name.as_str(),
                target,
                mode,
                args,
                user,
            ))
        },
    )
}

fn dispatch<'a, F>(
    scripts: impl Iterator<Item = &'a Script>,
    server: Option<&Server>,
    key: &str,
    mut call: F,
) -> Vec<Action>
where
    F: FnMut(&Lua, &Function, &AnyUserData) -> mlua::Result<()>,
{
    let mut actions = vec![];

    for script in scripts {
        let Some(lua) = script.lua() else {
            continue;
        };

        let (Some(context), Some(callback)) =
            (create_runtime(lua), lookup_callback(lua, key))
        else {
            continue;
        };

        if let Err(error) = call(lua, &callback, &context) {
            log::error!(
                "script {} hook failed for {} ({}): {error}",
                key,
                &script.name,
                &script.path.display(),
            );
            continue;
        }

        let mut context = match context.borrow_mut::<Api>() {
            Ok(context) => context,
            Err(error) => {
                log::error!("failed to borrow script runtime: {error}");
                continue;
            }
        };

        for action in context.take_actions() {
            match action {
                ApiAction::Command(command) => {
                    let Some(server) = server else {
                        log::warn!(
                            "script {key} requested command without server context"
                        );
                        continue;
                    };

                    actions.push(Action::Command {
                        server: server.clone(),
                        command,
                    });
                }
                ApiAction::Notification { name, title, body } => {
                    let Some(server) = server else {
                        log::warn!(
                            "script {key} requested notification without server context"
                        );
                        continue;
                    };

                    actions.push(Action::Notification {
                        server: server.clone(),
                        name,
                        title,
                        body,
                    });
                }
            }
        }
    }

    actions
}

fn create_runtime(lua: &Lua) -> Option<AnyUserData> {
    match lua.create_userdata(Api::new()) {
        Ok(context) => Some(context),
        Err(error) => {
            log::error!("failed to create script runtime: {error}");
            None
        }
    }
}

fn lookup_callback(lua: &Lua, key: &str) -> Option<Function> {
    lua.globals().get::<Function>(key).ok()
}

fn create_user(lua: &Lua, user: &User) -> Option<Table> {
    let table = lua.create_table().ok()?;

    table.set("nick", user.nickname().to_string()).ok()?;
    table.set("username", user.username()).ok()?;
    table.set("hostname", user.hostname()).ok()?;

    Some(table)
}
