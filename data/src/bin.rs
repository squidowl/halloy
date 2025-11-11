use data::bouncer::BouncerNetwork;
use data::message;
use data::server::Server;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server {
        name: "Highlight Server".into(),
        network: None,
    };

    let mut messages = message::tests::SERDE_IRC_MESSAGES
        .iter()
        .flat_map(|irc_message| {
            let (message, highlight) =
                message::tests::message_with_highlight_from_irc_message(
                    irc_message,
                    &server,
                );
            if let Some(highlight) =
                highlight.map(|highlight| highlight.message)
            {
                vec![message, highlight]
            } else {
                vec![message]
            }
        })
        .collect::<Vec<message::Message>>();

    messages.extend(
        message::tests::serde_broadcasts()
            .into_iter()
            .flat_map(message::tests::messages_from_broadcast),
    );

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
            if let (_, Some(message::Highlight { message, .. })) =
                message::tests::message_with_highlight_from_irc_message(
                    irc_message,
                    &bouncer_server,
                )
            {
                Some(message)
            } else {
                None
            }
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
