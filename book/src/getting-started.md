# Getting started

To get started with Halloy, you need to connect to at least one IRC server. The template config file has been set up with the [Libera](https://libera.chat/) server. However, there are many other servers available: [OFTC](https://www.oftc.net/), [Undernet](https://www.undernet.org/), [QuakeNet](https://www.quakenet.org/) and [many more](https://netsplit.de/networks/). Halloy can connect to multiple servers at the same time.

Once connected to a server, you can join channels. This can be done automatically from the config file or manually using the join command: `/join #channel`[^1]. To find channels, you can either use the list command: `/list`, or [browse for channels online](https://netsplit.de/channels/).

> 💡 Configuration in Halloy happens through a `config.toml` file. See [Configuration](./configuration.md).

Here are a few useful IRC commands for a new user[^2]

| Command           | Example                | Description                                |
| ----------------- | ---------------------- | ------------------------------------------ |
| `/join`           | `/join #halloy`        | Join a new channel                         |
| `/part`           | `/part #halloy`        | Part a channel                             |
| `/nick`           | `/nick halloyisgreat`  | Change your nickname                       |
| `/whois nickname` | `/whois halloyisgreat` | Displays information of nickname requested |
| `/list *keyword*` | `/list *linux*`        | List channels. Keyword is optional         |


[^1]: Channel names always start with a `#` symbol and do not contain spaces.
[^2]: Find more commands [here](https://en.wikipedia.org/wiki/List_of_Internet_Relay_Chat_commands).