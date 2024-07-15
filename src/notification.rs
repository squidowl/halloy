use data::{
    config::notification,
    user::{Nick, NickRef},
};

pub use toast::prepare;

pub mod audio;
mod toast;

pub fn connected(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(notification, audio, "Connected", server);
}

pub fn reconnected(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(notification, audio, "Reconnected", server);
}

pub fn disconnected(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    server: impl ToString,
) {
    show_notification(notification, audio, "Disconnected", server);
}

pub fn highlight(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    nick: NickRef,
    channel: String,
) {
    show_notification(
        notification,
        audio,
        "Highlight",
        format!("{} highlighted you in {}", nick, channel),
    );
}

pub fn file_transfer_request(
    notification: &notification::Loaded,
    audio: &mut audio::State,
    nick: Nick,
    server: impl ToString,
) {
    show_notification(
        notification,
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
