use data::{
    audio::Sound,
    config::{self, notification},
    user::{Nick, NickRef},
};

use crate::audio;

pub use self::toast::prepare;

mod toast;

pub fn connected(
    config: &config::Notifications<Sound>,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(&config.connected, audio, "Connected", server);
}

pub fn reconnected(
    config: &config::Notifications<Sound>,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(&config.reconnected, audio, "Reconnected", server);
}

pub fn disconnected(
    config: &config::Notifications<Sound>,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(&config.disconnected, audio, "Disconnected", server);
}

pub fn highlight(
    config: &config::Notifications<Sound>,
    audio: &mut audio::State,
    nick: NickRef,
    channel: String,
) {
    show_notification(
        &config.highlight,
        audio,
        "Highlight",
        format!("{} highlighted you in {}", nick, channel),
    );
}

pub fn file_transfer_request(
    config: &config::Notifications<Sound>,
    audio: &mut audio::State,
    nick: Nick,
    server: impl ToString,
) {
    show_notification(
        &config.file_transfer_request,
        audio,
        &format!("File transfer from {}", nick),
        server,
    );
}

fn show_notification(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    title: &str,
    body: impl ToString,
) {
    if notification.show_toast {
        toast::show(title, body);
    }

    if let Some(sound) = &notification.sound {
        audio.play(sound);
    }
}
