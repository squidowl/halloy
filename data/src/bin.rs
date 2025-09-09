use data::bouncer::BouncerNetwork;
use data::message;
use data::server::Server;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = message::tests::SERDE_IRC_MESSAGES
        .iter()
        .map(|irc_message| {
            message::tests::message_from_irc_message(irc_message)
        })
        .collect::<Vec<message::Message>>();

    messages.extend(
        message::tests::serde_broadcasts()
            .into_iter()
            .flat_map(message::tests::messages_from_broadcast),
    );

    let server = Server {
        name: "Highlight Server".into(),
        network: None,
    };

    messages.extend(message::tests::SERDE_IRC_MESSAGES.iter().filter_map(
        |irc_message| {
            message::tests::message_from_irc_message(irc_message)
                .into_highlight(server.clone())
                .map(|(highlight, _, _, _)| highlight)
        },
    ));

    let bouncer_server = Server {
        name: "Bounced Highlight Server".into(),
        network: Some(
            BouncerNetwork {
                id: "BouncerNetid".to_string(),
                name: "Bouncer Name".to_string(),
            }
            .into(),
        ),
    };

    messages.extend(message::tests::SERDE_IRC_MESSAGES.iter().filter_map(
        |irc_message| {
            message::tests::message_from_irc_message(irc_message)
                .into_highlight(bouncer_server.clone())
                .map(|(highlight, _, _, _)| highlight)
        },
    ));

    println!(
        "{}",
        serde_json::to_string_pretty(&messages).unwrap_or_else(|_| {
            panic!("unable to serialize messages {:?}", messages)
        })
    );

    Ok(())
}
