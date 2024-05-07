use data::{
    config::notification::Notification,
    user::{Nick, NickRef},
};

pub use toast::prepare;

pub mod audio;
mod toast;

pub fn connected(notification: &Notification, audio: &mut audio::State, server: impl ToString) {
    show_notification(notification, audio, "Connected", server);
}

pub fn reconnected(notification: &Notification, audio: &mut audio::State, server: impl ToString) {
    show_notification(notification, audio, "Reconnected", server);
}

pub fn disconnected(notification: &Notification, audio: &mut audio::State, server: impl ToString) {
    show_notification(notification, audio, "Disconnected", server);
}

pub fn highlight(
    notification: &Notification,
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
    notification: &Notification,
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
    notification: &Notification,
    audio: &mut audio::State,
    title: &str,
    body: impl ToString,
) {
    if notification.show_toast {
        toast::show(title, body);
    }

    let _ = audio.play(&notification.sound);
}
