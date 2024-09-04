use data::{
    audio::Sound,
    config::{self, notification},
    user::{Nick, NickRef},
};

use crate::audio;

pub use self::toast::prepare;

mod toast;

pub fn connected(config: &config::Notifications<Sound>, server: impl ToString) {
    show_notification(&config.connected, "Connected", server);
}

pub fn reconnected(config: &config::Notifications<Sound>, server: impl ToString) {
    show_notification(&config.reconnected, "Reconnected", server);
}

pub fn disconnected(config: &config::Notifications<Sound>, server: impl ToString) {
    show_notification(&config.disconnected, "Disconnected", server);
}

pub fn direct_message(config: &config::Notifications<Sound>, nick: NickRef) {
    show_notification(
        &config.direct_message,
        "Direct message",
        format!("{} sent you a direct message", nick),
    );
}

pub fn highlight(config: &config::Notifications<Sound>, nick: NickRef, channel: String) {
    show_notification(
        &config.highlight,
        "Highlight",
        format!("{} highlighted you in {}", nick, channel),
    );
}

pub fn file_transfer_request(
    config: &config::Notifications<Sound>,
    nick: Nick,
    server: impl ToString,
) {
    show_notification(
        &config.file_transfer_request,
        &format!("File transfer from {}", nick),
        server,
    );
}

pub fn monitored_online(config: &config::Notifications<Sound>, nick: Nick, server: impl ToString) {
    show_notification(
        &config.monitored_online,
        &format!("{} is online", nick),
        server,
    );
}

pub fn monitored_offline(config: &config::Notifications<Sound>, nick: Nick, server: impl ToString) {
    show_notification(
        &config.monitored_offline,
        &format!("{} is offline", nick),
        server,
    );
}

fn show_notification(notification: &notification::Loaded, title: &str, body: impl ToString) {
    if notification.show_toast {
        toast::show(title, body);
    }

    if let Some(sound) = &notification.sound {
        audio::play(sound.clone());
    }
}
