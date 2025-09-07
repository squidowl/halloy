use data::message;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut messages = message::tests::SERDE_IRC_MESSAGES
        .iter()
        .map(|irc_message| {
            message::tests::message_from_irc_message(irc_message)
        })
        .collect::<Vec<message::Message>>();

    for broadcast in message::tests::serde_broadcasts() {
        messages
            .append(&mut message::tests::messages_from_broadcast(broadcast));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&messages).unwrap_or_else(|_| {
            panic!("unable to serialize messages {:?}", messages)
        })
    );

    Ok(())
}
