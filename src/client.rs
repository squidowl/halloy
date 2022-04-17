use iced_native::event::{self, Event, MacOS, PlatformSpecific};
use iced_native::subscription;
use iced_native::{Hasher, Subscription};

use futures::stream::{BoxStream, StreamExt};
use irc::client::Sender;
use irc::proto::Message;

#[derive(Default, Debug)]
pub struct Client {
    pub streams: Vec<irc::client::ClientStream>,
    pub senders: Vec<irc::client::Sender>,
}

impl Client {
    pub async fn setup(configs: Vec<irc::client::data::Config>) -> irc::error::Result<Self> {
        let mut streams = Vec::new();
        let mut senders = Vec::new();

        for config in configs {
            // Immediate errors like failure to resolve the server's domain or to establish any connection will
            // manifest here in the result of prepare_client_and_connect.
            let mut client = irc::client::Client::from_config(config).await?;
            client.identify()?;

            streams.push(client.stream()?);
            senders.push(client.sender());
        }

        // https://github.com/aatxe/irc/blob/develop/examples/multiserver.rs

        // loop {
        //     let (message, index, _) =
        //         futures::future::select_all(streams.iter_mut().map(|s| s.select_next_some())).await;
        //     let message = message?;
        //     let sender = &senders[index];
        //     // process_msg(sender, message)?;
        // }

        Ok(Client { streams, senders })
    }

    // pub fn on_message(&self) -> Subscription<(Sender, Message)> {
    //     Subscription::from_recipe(OnMessage)
    // }
}

struct OnMessage;
